// This is some nasty shit, I admit.
// We cannot use PipelineCache from App World natively, so I tried my best to create one that's available from App World
// If only bevy could allow use to use PipelineCache from App World...:)

// https://github.com/bevyengine/bevy/blob/release-0.11.3/crates/bevy_render/src/render_resource/pipeline_cache.rs

use std::borrow::Cow;
use std::iter::FusedIterator;
use std::mem;

use bevy::prelude::*;
use bevy::render::render_resource::{
    BindGroupLayout, BindGroupLayoutId, CachedPipelineState, ComputePipeline,
    ComputePipelineDescriptor, ErasedPipelineLayout, ErasedShaderModule, Pipeline,
    PipelineCacheError, ShaderDefVal, ShaderImport, Source,
};
use bevy::render::renderer::RenderDevice;
use bevy::utils::{Entry, HashMap, HashSet};
use naga::valid::{Capabilities, ShaderStages};
use parking_lot::Mutex;
#[cfg(feature = "shader_format_spirv")]
use wgpu::util::make_spirv;
use wgpu::{
    Features, PipelineCompilationOptions, PipelineLayout, PipelineLayoutDescriptor,
    PushConstantRange, ShaderModuleDescriptor,
};

pub struct CachedAppPipeline {
    state: CachedPipelineState,
    descriptor: Box<ComputePipelineDescriptor>,
}

/// Index of a cached compute pipeline in a [`PipelineCache`].
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct CachedAppComputePipelineId(usize);

impl CachedAppComputePipelineId {
    /// An invalid cached compute pipeline index, often used to initialize a variable.
    pub const _INVALID: Self = CachedAppComputePipelineId(usize::MAX);

    #[inline]
    pub fn _id(&self) -> usize {
        self.0
    }
}

#[derive(Default)]
struct ShaderData {
    pipelines: HashSet<CachedAppComputePipelineId>,
    processed_shaders: HashMap<Vec<ShaderDefVal>, ErasedShaderModule>,
    resolved_imports: HashMap<ShaderImport, AssetId<Shader>>,
    dependents: HashSet<AssetId<Shader>>,
}

#[derive(Default)]
struct ShaderCache {
    data: HashMap<AssetId<Shader>, ShaderData>,
    shaders: HashMap<AssetId<Shader>, Shader>,
    import_path_shaders: HashMap<ShaderImport, AssetId<Shader>>,
    waiting_on_import: HashMap<ShaderImport, Vec<AssetId<Shader>>>,
    composer: naga_oil::compose::Composer,
}

impl ShaderCache {
    fn new(render_device: &RenderDevice) -> Self {
        const CAPABILITIES: &[(Features, Capabilities)] = &[
            (Features::PUSH_CONSTANTS, Capabilities::PUSH_CONSTANT),
            (Features::SHADER_F64, Capabilities::FLOAT64),
            (
                Features::SHADER_PRIMITIVE_INDEX,
                Capabilities::PRIMITIVE_INDEX,
            ),
            (
                Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                Capabilities::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
            ),
            (
                Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                Capabilities::SAMPLER_NON_UNIFORM_INDEXING,
            ),
            (
                Features::UNIFORM_BUFFER_AND_STORAGE_TEXTURE_ARRAY_NON_UNIFORM_INDEXING,
                Capabilities::UNIFORM_BUFFER_AND_STORAGE_TEXTURE_ARRAY_NON_UNIFORM_INDEXING,
            ),
        ];
        let features = render_device.features();
        let mut capabilities = Capabilities::empty();
        for (feature, capability) in CAPABILITIES {
            if features.contains(*feature) {
                capabilities |= *capability;
            }
        }

        #[cfg(debug_assertions)]
        let composer = naga_oil::compose::Composer::default();
        #[cfg(not(debug_assertions))]
        let composer = naga_oil::compose::Composer::non_validating();

        let composer = composer.with_capabilities(capabilities, ShaderStages::COMPUTE);

        Self {
            composer,
            data: Default::default(),
            shaders: Default::default(),
            import_path_shaders: Default::default(),
            waiting_on_import: Default::default(),
        }
    }

    fn add_import_to_composer(
        composer: &mut naga_oil::compose::Composer,
        import_path_shaders: &HashMap<ShaderImport, AssetId<Shader>>,
        shaders: &HashMap<AssetId<Shader>, Shader>,
        import: &ShaderImport,
    ) -> Result<(), PipelineCacheError> {
        if !composer.contains_module(&import.module_name()) {
            if let Some(shader_asset_id) = import_path_shaders.get(import) {
                if let Some(shader) = shaders.get(shader_asset_id) {
                    for import in &shader.imports {
                        Self::add_import_to_composer(
                            composer,
                            import_path_shaders,
                            shaders,
                            import,
                        )?;
                    }

                    composer.add_composable_module(shader.into())?;
                }
            }
            // if we fail to add a module the composer will tell us what is missing
        }

        Ok(())
    }

    fn get(
        &mut self,
        render_device: &RenderDevice,
        pipeline: CachedAppComputePipelineId,
        shader_asset_id: &AssetId<Shader>,
        shader_defs: &[ShaderDefVal],
    ) -> Result<ErasedShaderModule, PipelineCacheError> {
        let shader = self
            .shaders
            .get(shader_asset_id)
            .ok_or(PipelineCacheError::ShaderNotLoaded(*shader_asset_id))?;
        let data = self.data.entry(*shader_asset_id).or_default();
        let n_asset_imports = shader
            .imports()
            .filter(|import| matches!(import, ShaderImport::AssetPath(_)))
            .count();
        let n_resolved_asset_imports = data
            .resolved_imports
            .keys()
            .filter(|import| matches!(import, ShaderImport::AssetPath(_)))
            .count();
        if n_asset_imports != n_resolved_asset_imports {
            return Err(PipelineCacheError::ShaderImportNotYetAvailable);
        }

        data.pipelines.insert(pipeline);

        // PERF: this shader_defs clone isn't great. use raw_entry_mut when it stabilizes
        let module = match data.processed_shaders.entry(shader_defs.to_vec()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let mut shader_defs = shader_defs.to_vec();
                #[cfg(feature = "webgl")]
                {
                    shader_defs.push("NO_ARRAY_TEXTURES_SUPPORT".into());
                    shader_defs.push("SIXTEEN_BYTE_ALIGNMENT".into());
                }

                shader_defs.push(ShaderDefVal::UInt(
                    String::from("AVAILABLE_STORAGE_BUFFER_BINDINGS"),
                    render_device.limits().max_storage_buffers_per_shader_stage,
                ));

                debug!(
                    "processing shader {:?}, with shader defs {:?}",
                    shader_asset_id, shader_defs
                );
                let shader_source = match &shader.source {
                    #[cfg(feature = "shader_format_spirv")]
                    Source::SpirV(data) => make_spirv(data),
                    #[cfg(not(feature = "shader_format_spirv"))]
                    Source::SpirV(_) => {
                        unimplemented!(
                            "Enable feature \"shader_format_spirv\" to use SPIR-V shaders"
                        )
                    }
                    _ => {
                        for import in shader.imports() {
                            Self::add_import_to_composer(
                                &mut self.composer,
                                &self.import_path_shaders,
                                &self.shaders,
                                import,
                            )?;
                        }

                        let shader_defs = shader_defs
                            .into_iter()
                            .map(|def| match def {
                                ShaderDefVal::Bool(k, v) => {
                                    (k, naga_oil::compose::ShaderDefValue::Bool(v))
                                }
                                ShaderDefVal::Int(k, v) => {
                                    (k, naga_oil::compose::ShaderDefValue::Int(v))
                                }
                                ShaderDefVal::UInt(k, v) => {
                                    (k, naga_oil::compose::ShaderDefValue::UInt(v))
                                }
                            })
                            .collect::<std::collections::HashMap<_, _>>();

                        let naga = self.composer.make_naga_module(
                            naga_oil::compose::NagaModuleDescriptor {
                                shader_defs,
                                ..shader.into()
                            },
                        )?;

                        wgpu::ShaderSource::Naga(Cow::Owned(naga))
                    }
                };

                let module_descriptor = ShaderModuleDescriptor {
                    label: None,
                    source: shader_source,
                };

                render_device
                    .wgpu_device()
                    .push_error_scope(wgpu::ErrorFilter::Validation);
                let shader_module = render_device.create_shader_module(module_descriptor);
                let error = render_device.wgpu_device().pop_error_scope();

                // `now_or_never` will return Some if the future is ready and None otherwise.
                // On native platforms, wgpu will yield the error immediately while on wasm it may take longer since the browser APIs are asynchronous.
                // So to keep the complexity of the ShaderCache low, we will only catch this error early on native platforms,
                // and on wasm the error will be handled by wgpu and crash the application.
                if let Some(Some(wgpu::Error::Validation { description, .. })) =
                    bevy::utils::futures::now_or_never(error)
                {
                    return Err(PipelineCacheError::CreateShaderModule(description));
                }

                entry.insert(ErasedShaderModule::new(shader_module))
            }
        };

        Ok(module.clone())
    }

    fn clear(&mut self, shader_asset_id: &AssetId<Shader>) -> Vec<CachedAppComputePipelineId> {
        let mut shaders_to_clear = vec![*shader_asset_id];
        let mut pipelines_to_queue = Vec::new();
        while let Some(shader_asset_id) = shaders_to_clear.pop() {
            if let Some(data) = self.data.get_mut(&shader_asset_id) {
                data.processed_shaders.clear();
                pipelines_to_queue.extend(data.pipelines.iter().cloned());
                shaders_to_clear.extend(data.dependents.iter().copied());
            }
        }

        pipelines_to_queue
    }

    fn set_shader(
        &mut self,
        shader_asset_id: &AssetId<Shader>,
        shader: Shader,
    ) -> Vec<CachedAppComputePipelineId> {
        let pipelines_to_queue = self.clear(shader_asset_id);
        let path = shader.import_path();
        self.import_path_shaders
            .insert(path.clone(), *shader_asset_id);
        if let Some(waiting_shaders) = self.waiting_on_import.get_mut(path) {
            for waiting_shader in waiting_shaders.drain(..) {
                // resolve waiting shader import
                let data = self.data.entry(waiting_shader).or_default();
                data.resolved_imports.insert(path.clone(), *shader_asset_id);
                // add waiting shader as dependent of this shader
                let data = self.data.entry(*shader_asset_id).or_default();
                data.dependents.insert(waiting_shader);
            }
        }

        for import in shader.imports() {
            if let Some(import_handle) = self.import_path_shaders.get(import) {
                // resolve import because it is currently available
                let data = self.data.entry(*shader_asset_id).or_default();
                data.resolved_imports.insert(import.clone(), *import_handle);
                // add this shader as a dependent of the import
                let data = self.data.entry(*import_handle).or_default();
                data.dependents.insert(*shader_asset_id);
            } else {
                let waiting = self.waiting_on_import.entry(import.clone()).or_default();
                waiting.push(*shader_asset_id);
            }
        }

        self.shaders.insert(*shader_asset_id, shader);
        pipelines_to_queue
    }

    fn remove(&mut self, shader_asset_id: &AssetId<Shader>) -> Vec<CachedAppComputePipelineId> {
        let pipelines_to_queue = self.clear(shader_asset_id);
        if let Some(shader) = self.shaders.remove(shader_asset_id) {
            self.import_path_shaders.remove(shader.import_path());
        }

        pipelines_to_queue
    }
}

type LayoutCacheKey = (Vec<BindGroupLayoutId>, Vec<PushConstantRange>);
#[derive(Default, Clone)]
struct LayoutCache {
    layouts: HashMap<LayoutCacheKey, ErasedPipelineLayout>,
}

impl LayoutCache {
    fn get(
        &mut self,
        render_device: &RenderDevice,
        bind_group_layouts: &[BindGroupLayout],
        push_constant_ranges: Vec<PushConstantRange>,
    ) -> &PipelineLayout {
        let bind_group_ids = bind_group_layouts.iter().map(|l| l.id()).collect();
        self.layouts
            .entry((bind_group_ids, push_constant_ranges))
            .or_insert_with_key(|(_, push_constant_ranges)| {
                let bind_group_layouts = bind_group_layouts
                    .iter()
                    .map(|l| l.value())
                    .collect::<Vec<_>>();
                ErasedPipelineLayout::new(render_device.create_pipeline_layout(
                    &PipelineLayoutDescriptor {
                        bind_group_layouts: &bind_group_layouts,
                        push_constant_ranges,
                        ..default()
                    },
                ))
            })
    }
}

#[derive(Resource)]
pub struct AppPipelineCache {
    layout_cache: LayoutCache,
    shader_cache: ShaderCache,
    device: RenderDevice,
    pipelines: Vec<CachedAppPipeline>,
    waiting_pipelines: HashSet<CachedAppComputePipelineId>,
    new_pipelines: Mutex<Vec<CachedAppPipeline>>,
}

impl AppPipelineCache {
    pub fn new(device: RenderDevice) -> Self {
        Self {
            shader_cache: ShaderCache::new(&device),
            device,
            layout_cache: default(),
            waiting_pipelines: default(),
            new_pipelines: default(),
            pipelines: default(),
        }
    }

    pub fn queue_app_compute_pipeline(
        &self,
        descriptor: ComputePipelineDescriptor,
    ) -> CachedAppComputePipelineId {
        let mut new_pipelines = self.new_pipelines.lock();
        let id = CachedAppComputePipelineId(self.pipelines.len() + new_pipelines.len());
        new_pipelines.push(CachedAppPipeline {
            descriptor: Box::new(descriptor),
            state: CachedPipelineState::Queued,
        });
        id
    }

    pub fn process_queue(&mut self) {
        let mut waiting_pipelines = mem::take(&mut self.waiting_pipelines);
        let mut pipelines = mem::take(&mut self.pipelines);

        {
            let mut new_pipelines = self.new_pipelines.lock();
            for new_pipeline in new_pipelines.drain(..) {
                let id = pipelines.len();
                pipelines.push(new_pipeline);
                waiting_pipelines.insert(CachedAppComputePipelineId(id));
            }
        }

        for id in waiting_pipelines {
            let pipeline = &mut pipelines[id.0];
            if matches!(pipeline.state, CachedPipelineState::Ok(_)) {
                continue;
            }

            pipeline.state = self.process_compute_pipeline(id, &pipeline.descriptor);

            if let CachedPipelineState::Err(err) = &pipeline.state {
                match err {
                    PipelineCacheError::ShaderNotLoaded(_)
                    | PipelineCacheError::ShaderImportNotYetAvailable => {
                        // retry
                        self.waiting_pipelines.insert(id);
                    }
                    // shader could not be processed ... retrying won't help
                    PipelineCacheError::ProcessShaderError(err) => {
                        let error_detail = err.emit_to_string(&self.shader_cache.composer);
                        error!("failed to process shader:\n{}", error_detail);
                        continue;
                    }
                    PipelineCacheError::CreateShaderModule(description) => {
                        error!("failed to create shader module: {}", description);
                        continue;
                    }
                }
            }
        }

        self.pipelines = pipelines;
    }

    fn process_compute_pipeline(
        &mut self,
        id: CachedAppComputePipelineId,
        descriptor: &ComputePipelineDescriptor,
    ) -> CachedPipelineState {
        let layout = if descriptor.layout.is_empty() && descriptor.push_constant_ranges.is_empty() {
            None
        } else {
            Some(self.layout_cache.get(
                &self.device,
                &descriptor.layout,
                descriptor.push_constant_ranges.to_vec(),
            ))
        };

        let compute_module = match self.shader_cache.get(
            &self.device,
            id,
            &descriptor.shader.id(),
            &descriptor.shader_defs,
        ) {
            Ok(module) => module,
            Err(err) => {
                return CachedPipelineState::Err(err);
            }
        };

        let descriptor = wgpu::ComputePipelineDescriptor {
            compilation_options: PipelineCompilationOptions::default(), // changed
            label: descriptor.label.as_deref(),
            layout,
            module: &compute_module,
            entry_point: descriptor.entry_point.as_ref(),
        };

        let pipeline = self.device.create_compute_pipeline(&descriptor);

        CachedPipelineState::Ok(Pipeline::ComputePipeline(pipeline))
    }

    #[inline]
    pub fn get_compute_pipeline(&self, id: CachedAppComputePipelineId) -> Option<&ComputePipeline> {
        if self.pipelines.len() <= id.0 {
            return None;
        }

        if let CachedPipelineState::Ok(Pipeline::ComputePipeline(pipeline)) =
            &self.pipelines[id.0].state
        {
            Some(pipeline)
        } else {
            None
        }
    }

    pub fn set_shader(&mut self, shader_asset_id: &AssetId<Shader>, shader: &Shader) {
        let pipelines_to_queue = self
            .shader_cache
            .set_shader(shader_asset_id, shader.clone());
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline.0].state = CachedPipelineState::Queued;
            self.waiting_pipelines.insert(cached_pipeline);
        }
    }

    pub fn remove_shader(&mut self, shader: &AssetId<Shader>) {
        let pipelines_to_queue = self.shader_cache.remove(shader);
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline.0].state = CachedPipelineState::Queued;
            self.waiting_pipelines.insert(cached_pipeline);
        }
    }
}

struct ErrorSources<'a> {
    current: Option<&'a (dyn std::error::Error + 'static)>,
}

#[allow(dead_code)]
impl<'a> ErrorSources<'a> {
    fn of(error: &'a dyn std::error::Error) -> Self {
        Self {
            current: error.source(),
        }
    }
}

impl<'a> Iterator for ErrorSources<'a> {
    type Item = &'a (dyn std::error::Error + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        self.current = self.current.and_then(std::error::Error::source);
        current
    }
}

impl FusedIterator for ErrorSources<'_> {}
