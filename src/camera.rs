use bevy::{
    input::mouse::{AccumulatedMouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
};

#[derive(Debug, Clone, Default, Component)]
pub struct AtlasCamera;

#[derive(Debug, Clone, Component)]
pub struct OrbitCamera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::new(0.0, 0.7, 0.0),
            distance: 22.0,
            yaw: 0.72,
            pitch: -0.68,
        }
    }
}

#[derive(Debug, Default, Resource)]
pub struct ViewportInputCapture {
    pub pointer: bool,
}

/// Camera scene and navigation only. The scene is expressed with Bevy 0.19's
/// BSN so its ECS composition is declarative and patchable.
pub struct MapCameraPlugin;

impl Plugin for MapCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewportInputCapture>()
            .add_systems(Startup, viewport_scene.spawn())
            .add_systems(Update, orbit_camera);
    }
}

fn viewport_scene() -> impl SceneList {
    bsn_list![
        (
            #Sun
            DirectionalLight {
                illuminance: 18_000.0,
                shadow_maps_enabled: false,
            }
            template_value(
                Transform::from_xyz(-8.0, 14.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y)
            )
        ),
        (
            #AtlasCamera
            Camera3d
            AtlasCamera
            OrbitCamera
            Transform
        ),
    ]
}

fn orbit_camera(
    motion: Res<AccumulatedMouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    buttons: Res<ButtonInput<MouseButton>>,
    capture: Res<ViewportInputCapture>,
    camera: Single<(&mut Transform, &mut OrbitCamera), With<AtlasCamera>>,
) {
    let (mut transform, mut orbit) = camera.into_inner();
    if !capture.pointer && buttons.pressed(MouseButton::Right) {
        orbit.yaw -= motion.delta.x * 0.006;
        orbit.pitch = (orbit.pitch - motion.delta.y * 0.006).clamp(-1.48, -0.08);
    }
    if !capture.pointer {
        for event in wheel.read() {
            let amount = match event.unit {
                MouseScrollUnit::Line => event.y,
                MouseScrollUnit::Pixel => event.y * 0.04,
            };
            orbit.distance = (orbit.distance * (-amount * 0.09).exp()).clamp(3.0, 100.0);
        }
    } else {
        for _ in wheel.read() {}
    }
    let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
    transform.translation = orbit.target + rotation * Vec3::new(0.0, 0.0, orbit.distance);
    transform.look_at(orbit.target, Vec3::Y);
}
