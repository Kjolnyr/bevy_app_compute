// This is some nasty shit, I admit.
// We cannot use PipelineCache from App World natively, so I tried my best to create one that's available from App World
// If only bevy could allow use to use PipelineCache from App World...:)

use std::iter::FusedIterator;
use std::mem;

use bevy::prelude::*;
use bevy::render::render_resource::{
    AsModuleDescriptorError, BindGroupLayout, BindGroupLayoutId, CachedPipelineState,
    ComputePipeline, ComputePipelineDescriptor, ErasedPipelineLayout, ErasedShaderModule, Pipeline,
    PipelineCacheError, PipelineLayout, PipelineLayoutDescriptor, ProcessedShader,
    PushConstantRange, ShaderDefVal, ShaderImport, ShaderProcessor, ShaderReflectError,
};
use bevy::render::renderer::RenderDevice;
use bevy::utils::{Entry, HashMap, HashSet};
use parking_lot::Mutex;
use wgpu;

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
    resolved_imports: HashMap<ShaderImport, Handle<Shader>>,
    dependents: HashSet<Handle<Shader>>,
}

#[derive(Default)]
struct ShaderCache {
    data: HashMap<Handle<Shader>, ShaderData>,
    shaders: HashMap<Handle<Shader>, Shader>,
    import_path_shaders: HashMap<ShaderImport, Handle<Shader>>,
    waiting_on_import: HashMap<ShaderImport, Vec<Handle<Shader>>>,
    processor: ShaderProcessor,
}

impl ShaderCache {
    fn get(
        &mut self,
        render_device: &RenderDevice,
        pipeline: CachedAppComputePipelineId,
        handle: &Handle<Shader>,
        shader_defs: &[ShaderDefVal],
    ) -> Result<ErasedShaderModule, PipelineCacheError> {
        let shader = self
            .shaders
            .get(handle)
            .ok_or_else(|| PipelineCacheError::ShaderNotLoaded(handle.clone_weak()))?;
        let data = self.data.entry(handle.clone_weak()).or_default();
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
                    handle, shader_defs
                );
                let processed = self.processor.process(
                    shader,
                    &shader_defs,
                    &self.shaders,
                    &self.import_path_shaders,
                )?;
                let module_descriptor = match processed
                    .get_module_descriptor(render_device.features())
                {
                    Ok(module_descriptor) => module_descriptor,
                    Err(err) => {
                        return Err(PipelineCacheError::AsModuleDescriptorError(err, processed));
                    }
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

    fn clear(&mut self, handle: &Handle<Shader>) -> Vec<CachedAppComputePipelineId> {
        let mut shaders_to_clear = vec![handle.clone_weak()];
        let mut pipelines_to_queue = Vec::new();
        while let Some(handle) = shaders_to_clear.pop() {
            if let Some(data) = self.data.get_mut(&handle) {
                data.processed_shaders.clear();
                pipelines_to_queue.extend(data.pipelines.iter().cloned());
                shaders_to_clear.extend(data.dependents.iter().map(|h| h.clone_weak()));
            }
        }

        pipelines_to_queue
    }

    fn set_shader(
        &mut self,
        handle: &Handle<Shader>,
        shader: Shader,
    ) -> Vec<CachedAppComputePipelineId> {
        let pipelines_to_queue = self.clear(handle);
        if let Some(path) = shader.import_path() {
            self.import_path_shaders
                .insert(path.clone(), handle.clone_weak());
            if let Some(waiting_shaders) = self.waiting_on_import.get_mut(path) {
                for waiting_shader in waiting_shaders.drain(..) {
                    // resolve waiting shader import
                    let data = self.data.entry(waiting_shader.clone_weak()).or_default();
                    data.resolved_imports
                        .insert(path.clone(), handle.clone_weak());
                    // add waiting shader as dependent of this shader
                    let data = self.data.entry(handle.clone_weak()).or_default();
                    data.dependents.insert(waiting_shader.clone_weak());
                }
            }
        }

        for import in shader.imports() {
            if let Some(import_handle) = self.import_path_shaders.get(import) {
                // resolve import because it is currently available
                let data = self.data.entry(handle.clone_weak()).or_default();
                data.resolved_imports
                    .insert(import.clone(), import_handle.clone_weak());
                // add this shader as a dependent of the import
                let data = self.data.entry(import_handle.clone_weak()).or_default();
                data.dependents.insert(handle.clone_weak());
            } else {
                let waiting = self.waiting_on_import.entry(import.clone()).or_default();
                waiting.push(handle.clone_weak());
            }
        }

        self.shaders.insert(handle.clone_weak(), shader);
        pipelines_to_queue
    }

    fn remove(&mut self, handle: &Handle<Shader>) -> Vec<CachedAppComputePipelineId> {
        let pipelines_to_queue = self.clear(handle);
        if let Some(shader) = self.shaders.remove(handle) {
            if let Some(import_path) = shader.import_path() {
                self.import_path_shaders.remove(import_path);
            }
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
    render_device: RenderDevice,
    shader_cache: ShaderCache,
    layout_cache: LayoutCache,
    pipelines: Vec<CachedAppPipeline>,
    waiting_pipelines: HashSet<CachedAppComputePipelineId>,
    new_pipelines: Mutex<Vec<CachedAppPipeline>>,
}

impl AppPipelineCache {
    pub fn new(render_device: RenderDevice) -> Self {
        Self {
            render_device,
            shader_cache: default(),
            layout_cache: default(),
            pipelines: default(),
            waiting_pipelines: default(),
            new_pipelines: default(),
        }
    }

    pub fn queue_app_compute_pipeline(
        &self,
        descriptor: ComputePipelineDescriptor,
    ) -> CachedAppComputePipelineId {
        let mut new_pipelines = self.new_pipelines.lock();
        let id = CachedAppComputePipelineId(self.pipelines.len() + new_pipelines.len());
        new_pipelines.push(CachedAppPipeline {
            state: CachedPipelineState::Queued,
            descriptor: Box::new(descriptor),
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
                        error!("failed to process shader: {}", err);
                        continue;
                    }
                    PipelineCacheError::AsModuleDescriptorError(err, source) => {
                        log_shader_error(source, err);
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
                &self.render_device,
                &descriptor.layout,
                descriptor.push_constant_ranges.to_vec(),
            ))
        };

        let compute_module = match self.shader_cache.get(
            &self.render_device,
            id,
            &descriptor.shader,
            &descriptor.shader_defs,
        ) {
            Ok(module) => module,
            Err(err) => {
                return CachedPipelineState::Err(err);
            }
        };

        let descriptor = wgpu::ComputePipelineDescriptor {
            label: descriptor.label.as_deref(),
            layout,
            module: &compute_module,
            entry_point: descriptor.entry_point.as_ref(),
        };

        let pipeline = self.render_device.create_compute_pipeline(&descriptor);

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

    pub fn set_shader(&mut self, handle: &Handle<Shader>, shader: &Shader) {
        let pipelines_to_queue = self.shader_cache.set_shader(handle, shader.clone());
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline.0].state = CachedPipelineState::Queued;
            self.waiting_pipelines.insert(cached_pipeline);
        }
    }

    pub fn remove_shader(&mut self, shader: &Handle<Shader>) {
        let pipelines_to_queue = self.shader_cache.remove(shader);
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline.0].state = CachedPipelineState::Queued;
            self.waiting_pipelines.insert(cached_pipeline);
        }
    }
}

fn log_shader_error(source: &ProcessedShader, error: &AsModuleDescriptorError) {
    use codespan_reporting::{
        diagnostic::{Diagnostic, Label},
        files::SimpleFile,
        term,
    };

    match error {
        AsModuleDescriptorError::ShaderReflectError(error) => match error {
            ShaderReflectError::WgslParse(error) => {
                let source = source
                    .get_wgsl_source()
                    .expect("non-wgsl source for wgsl error");
                let msg = error.emit_to_string(source);
                error!("failed to process shader:\n{}", msg);
            }
            ShaderReflectError::GlslParse(errors) => {
                let source = source
                    .get_glsl_source()
                    .expect("non-glsl source for glsl error");
                let files = SimpleFile::new("glsl", source);
                let config = codespan_reporting::term::Config::default();
                let mut writer = term::termcolor::Ansi::new(Vec::new());

                for err in errors {
                    let mut diagnostic = Diagnostic::error().with_message(err.kind.to_string());

                    if let Some(range) = err.meta.to_range() {
                        diagnostic = diagnostic.with_labels(vec![Label::primary((), range)]);
                    }

                    term::emit(&mut writer, &config, &files, &diagnostic)
                        .expect("cannot write error");
                }

                let msg = writer.into_inner();
                let msg = String::from_utf8_lossy(&msg);

                error!("failed to process shader: \n{}", msg);
            }
            ShaderReflectError::SpirVParse(error) => {
                error!("failed to process shader:\n{}", error);
            }
            ShaderReflectError::Validation(error) => {
                let (filename, source) = match source {
                    ProcessedShader::Wgsl(source) => ("wgsl", source.as_ref()),
                    ProcessedShader::Glsl(source, _) => ("glsl", source.as_ref()),
                    ProcessedShader::SpirV(_) => {
                        error!("failed to process shader:\n{}", error);
                        return;
                    }
                };

                let files = SimpleFile::new(filename, source);
                let config = term::Config::default();
                let mut writer = term::termcolor::Ansi::new(Vec::new());

                let diagnostic = Diagnostic::error()
                    .with_message(error.to_string())
                    .with_labels(
                        error
                            .spans()
                            .map(|(span, desc)| {
                                Label::primary((), span.to_range().unwrap())
                                    .with_message(desc.to_owned())
                            })
                            .collect(),
                    )
                    .with_notes(
                        ErrorSources::of(error)
                            .map(|source| source.to_string())
                            .collect(),
                    );

                term::emit(&mut writer, &config, &files, &diagnostic).expect("cannot write error");

                let msg = writer.into_inner();
                let msg = String::from_utf8_lossy(&msg);

                error!("failed to process shader: \n{}", msg);
            }
        },
        AsModuleDescriptorError::WgslConversion(error) => {
            error!("failed to convert shader to wgsl: \n{}", error);
        }
        AsModuleDescriptorError::SpirVConversion(error) => {
            error!("failed to convert shader to spirv: \n{}", error);
        }
    }
}

struct ErrorSources<'a> {
    current: Option<&'a (dyn std::error::Error + 'static)>,
}

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

impl<'a> FusedIterator for ErrorSources<'a> {}
