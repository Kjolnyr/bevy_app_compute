use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension, StandardMaterial},
    prelude::*,
    render::{render_resource::AsBindGroup, storage::ShaderStorageBuffer},
};
use bevy_app_compute::prelude::*;
use tracing::info;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct SharedStorageMaterial {
    #[storage(100, read_only)]
    heights: Handle<ShaderStorageBuffer>,
}

impl MaterialExtension for SharedStorageMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/shared_storage_vertex.wgsl".into()
    }
}

#[derive(TypePath)]
struct SharedStorageComputeShader;

impl ComputeShader for SharedStorageComputeShader {
    fn shader() -> ShaderRef {
        "shaders/shared_storage_compute.wgsl".into()
    }
}

#[derive(Resource)]
struct SimpleSharedStorageComputeWorker;

impl ComputeWorker for SimpleSharedStorageComputeWorker {
    fn build(world: &mut World) -> AppComputeWorker<Self> {
        let heights = vec![0.0f32; 121];
        let worker = AppComputeWorkerBuilder::new(world)
            .add_uniform("time", &0.0f32)
            // This allows us to retrieve a `Handle<ShaderStorageBuffer>` which gets shared between the compute shader and the material on the GPU.
            .add_rw_storage_asset("heights", &heights)
            .add_pass::<SharedStorageComputeShader>([121, 1, 1], &["time"], &["heights"])
            .build();
        worker
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, SharedStorageMaterial>,
        >::default())
        .add_plugins(AppComputePlugin)
        .add_plugins(AppComputeWorkerPlugin::<SimpleSharedStorageComputeWorker>::default())
        .add_systems(Startup, startup_system)
        .add_systems(
            Update,
            (
                spawn_mesh_system
                    .run_if(resource_added::<AppComputeWorker<SimpleSharedStorageComputeWorker>>),
                update_shared_storage_time_system
                    .run_if(resource_exists::<AppComputeWorker<SimpleSharedStorageComputeWorker>>),
            ),
        )
        .run();
}

fn startup_system(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, -0.5, 0.0)),
    ));
}

fn spawn_mesh_system(
    mut commands: Commands,
    worker: Res<AppComputeWorker<SimpleSharedStorageComputeWorker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, SharedStorageMaterial>>>,
) {
    if let Some(heights_handle) = worker.get_storage_buffer_asset_handle("heights") {
        let mesh = meshes.add(Plane3d::default().mesh().size(4.0, 4.0).subdivisions(10));

        let material = materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(0.3, 0.7, 0.3),
                ..default()
            },
            extension: SharedStorageMaterial {
                heights: heights_handle.clone(),
            },
        });

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));

        info!("Terrain mesh spawned");
    }
}

fn update_shared_storage_time_system(
    time: Res<Time>,
    mut worker: ResMut<AppComputeWorker<SimpleSharedStorageComputeWorker>>,
) {
    worker.write("time", &time.elapsed_secs());
}
