use crate::main_camera::MainCamera;
use crate::{loading::TextureAssets, DynamicActorBundle, GameState, PhysicsLayers};
use bevy::{prelude::*, render::camera::RenderTarget};
use heron::prelude::*;

pub const PLAYER_CENTER: Vec2 = Vec2::new(0., 8.);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MouseData {
            world_pos: Vec2::ZERO,
            vec_from_player: Vec2::Y,
        })
        .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(spawn_player))
        .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(spawn_projectile))
        .add_system_set(SystemSet::on_update(GameState::Playing).with_system(mouse_input))
        .add_system_set(SystemSet::on_update(GameState::Playing).with_system(aim))
        .add_system_set(SystemSet::on_update(GameState::Playing).with_system(launch))
        .add_system_set(
            SystemSet::on_update(GameState::Playing).with_system(projectile_destruction),
        );
    }
}

#[derive(Component)]
struct Player;

#[derive(Component)]
pub(crate) struct PlayerProjectile {
    pub size: i32,
}

#[derive(Component)]
struct Charging {
    timer: Timer,
}

#[derive(Component)]
struct Fired;

#[derive(Default)]
struct MouseData {
    world_pos: Vec2,
    vec_from_player: Vec2,
}

fn spawn_player(mut commands: Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            // texture: textures.texture_bevy.clone(),
            sprite: Sprite {
                color: Color::RED,
                anchor: bevy::sprite::Anchor::BottomCenter,
                custom_size: Some(Vec2::new(0.75, 1.5)),
                ..Default::default()
            },
            transform: Transform::from_translation(PLAYER_CENTER.extend(0.)),
            ..Default::default()
        })
        .insert(Player);
}

fn spawn_projectile(mut commands: Commands, texture_assets: Res<TextureAssets>) {
    commands
        .spawn_bundle(SpriteBundle {
            texture: texture_assets.circle.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::ONE),
                color: Color::GREEN,
                ..Default::default()
            },
            transform: Transform::from_translation(Vec3::new(-10., 45., 0.)),
            ..Default::default()
        })
        .insert(PlayerProjectile { size: 1 })
        .insert_bundle(DynamicActorBundle {
            shape: CollisionShape::Sphere { radius: 0.5 },
            material: PhysicMaterial {
                friction: 0.,
                restitution: 1.5,
                ..Default::default()
            },
            layers: CollisionLayers::none()
                .with_group(PhysicsLayers::PlayerProj)
                .with_masks(&[
                    PhysicsLayers::Ground,
                    PhysicsLayers::Enemy,
                    PhysicsLayers::EnemyProj,
                ]),
            ..Default::default()
        });
}

fn aim(
    time: Res<Time>,
    mouse_data: Res<MouseData>,
    mut proj_query: Query<(&mut PlayerProjectile, &mut Transform, &mut Charging)>,
) {
    for (mut proj, mut proj_trans, mut charge) in proj_query.iter_mut() {
        charge.timer.tick(time.delta());
        let c = (charge.timer.elapsed_secs().sin().powi(2) * 4.) + 1.;
        proj.size = c.round() as i32;
        proj_trans.scale = Vec3::ONE * (0.5 + (0.05 * proj.size as f32));
        proj_trans.translation = (2. * mouse_data.vec_from_player + PLAYER_CENTER).extend(0.);
    }
}

fn launch(
    mut commands: Commands,
    mouse_data: Res<MouseData>,
    mut query: Query<(Entity, &mut Velocity), (With<PlayerProjectile>, With<Fired>)>,
) {
    for (entity, mut vel) in query.iter_mut() {
        vel.linear = mouse_data.vec_from_player.extend(0.) * 12.;

        commands.entity(entity).remove::<Fired>();
    }
}

fn mouse_input(
    windows: Res<Windows>,
    input: Res<Input<MouseButton>>,
    cam_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    proj_query: Query<Entity, With<PlayerProjectile>>,
    mut commands: Commands,
    mut mouse_data: ResMut<MouseData>,
    mut events: EventReader<CursorMoved>,
) {
    let (camera, camera_transform) = cam_query.single();

    let window = if let RenderTarget::Window(id) = camera.target {
        windows.get(id).unwrap()
    } else {
        windows.get_primary().unwrap()
    };

    for e in events.iter() {
        let window_size = Vec2::new(window.width() as f32, window.height() as f32);
        let ndc = (e.position / window_size) * 2. - Vec2::ONE;
        let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();
        mouse_data.world_pos = ndc_to_world.project_point3(ndc.extend(-1.)).truncate();
    }

    let a = Vec2::Y;
    let b = (mouse_data.world_pos - PLAYER_CENTER).normalize();
    let mut angle = ((a.x * b.x) + (a.y * b.y)).acos();
    let cross = (a.x * b.y) - (a.y * b.x);

    if cross < 0. {
        angle *= -1.;
    }

    mouse_data.vec_from_player = Vec2::from_angle(angle).rotate(Vec2::Y).normalize();

    if input.just_pressed(MouseButton::Left) {
        let entity = proj_query.single();
        commands
            .entity(entity)
            .remove::<RigidBody>()
            .insert(RigidBody::Static)
            .insert(Charging {
                timer: Timer::from_seconds(10., true),
            });
    } else if input.just_released(MouseButton::Left) {
        let entity = proj_query.single();
        commands
            .entity(entity)
            .remove::<RigidBody>()
            .remove::<Charging>()
            .insert(RigidBody::Dynamic)
            .insert(Fired);
    }
}

fn projectile_destruction(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &Collisions), With<PlayerProjectile>>,
) {
    for (entity, mut transform, collisions) in query.iter_mut() {
        for c in collisions.collision_data() {
            if c.collision_layers().contains_group(PhysicsLayers::Enemy)
                && !c
                    .collision_layers()
                    .contains_group(PhysicsLayers::EnemyProj)
            {
                transform.translation = Vec3::NEG_Y * 50.;
                commands.entity(entity).remove::<RigidBody>();
            }
        }
    }
}
