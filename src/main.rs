use bevy::{
	prelude::*,

	sprite::MaterialMesh2dBundle,

	window::{PresentMode, WindowResolution},
	input::common_conditions::input_toggle_active,
};

use bevy_inspector_egui::quick::WorldInspectorPlugin;

const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 720.0;
const TIME_STEP: f32 = 1.0 / 60.0;

const BUTTON_BG_COLOUR: Color = Color::rgb(0.15, 0.15, 0.15);
const BUTTON_GB_COLOUR_HOVERED: Color = Color::rgb(0.25, 0.25, 0.25);

const BACKGROUND_COLOUR: Color = Color::BLACK;

const BULLET_COLOUR: Color = Color::WHITE;
const BULLET_SPEED: f32 = 400.0;

const SCOREBOARD_FONT_SIZE: f32 = 24.0;
const SCOREBOARD_COLOUR: Color = Color::AZURE;

const PLAYER_SPEED: f32 = 500.0;

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, States)]
enum AppState {
	#[default]
	Menu,

	GameRunning,
	GameOver,
}

fn main() {
	App::new()
		.add_plugins(
			DefaultPlugins
			.set(WindowPlugin {
				primary_window: Some(Window {
					title: "Aidan's Competency Project - Bevy Invaders!".to_string(),
					resolution: WindowResolution::new(WIDTH, HEIGHT).with_scale_factor_override(1.0),
					present_mode: PresentMode::AutoVsync,
					fit_canvas_to_parent: true,
					prevent_default_event_handling: true,
					resizable: false,
					..default()
				}),
				..default()
			}).set(ImagePlugin::default_nearest())
		)
		.add_plugin(WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::F12)))
		// .insert_resource(Scoreboard { score: 0 })
		.insert_resource(ClearColor(BACKGROUND_COLOUR))
		.add_state::<AppState>()
		.add_startup_system(setup)
		.add_system(menu_setup.in_schedule(OnEnter(AppState::Menu)))
		.add_system(menu.in_set(OnUpdate(AppState::Menu)))
		.add_system(menu_cleanup.in_schedule(OnExit(AppState::Menu)))
		.add_system(game_setup.in_schedule(OnEnter(AppState::GameRunning)))
		.add_systems(
			(
				collision_check,
				apply_velocity.before(collision_check),
				// apply_velocity,
				remove_offscreen_entities.after(apply_velocity),
				move_player
					.before(collision_check)
					.after(apply_velocity),
				player_shoot,
				play_shooting_sound.after(player_shoot),
				update_scoreboard,
			).in_set(OnUpdate(AppState::GameRunning))
		)
		.add_event::<CollisionEvent>()
		.add_event::<ShootingEvent>()

		// Make the calculations run 60 times per second, making it separate from the framerate
		// otherwise janky stuff can happen at high framerates (looking at you, Skyrim)
		.insert_resource(FixedTime::new_from_secs(TIME_STEP))
		.insert_resource(Scoreboard { score: 0 })
		// .add_system(update_scoreboard)
		.add_system(bevy::window::close_on_esc)
		.run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Bullet;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Collider;

#[derive(Default)]
struct CollisionEvent;

#[derive(Default)]
struct ShootingEvent;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Menu;

#[derive(Resource)]
struct ShootingSound(Handle<AudioSource>);

#[derive(Resource)]
struct Scoreboard {
	score: usize,
}

fn setup(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
) {
		// Load the audio files and insert them into our resource
	// This stops us having to load the file from disk everytime we want to play the sound.

	// However, we don't need to do this for the player sprite, as there will only ever be 1
	// and so there won't be any associated performance cost.
	let shooting_sound = asset_server.load("audio/player_shoot.wav");
	commands.insert_resource(ShootingSound(shooting_sound));

	commands.spawn(Camera2dBundle::default());
}

fn menu_setup(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
) {
	commands.spawn(
		(
			NodeBundle {
				style: Style {
					size: Size::width(Val::Percent(100.0)),
					align_items: AlignItems::Center,
					justify_content: JustifyContent::Center,
					..default()
				},
				..default()
			},
			Menu,
		)
	)
	.with_children(|parent| {
		parent
			.spawn(ButtonBundle {
				style: Style {
					size: Size::new(Val::Px(300.0), Val::Px(65.0)),
					justify_content: JustifyContent::Center,
					align_items: AlignItems::Center,
					..default()
				},
				background_color: BUTTON_BG_COLOUR.into(),
				..default()
			})
			.with_children(|parent| {
				parent.spawn(TextBundle::from_section(
					"Start Game",
					TextStyle {
						font: asset_server.load("fonts/amiga4ever/amiga4ever.ttf"),
						font_size: 20.0,
						color: Color::rgb(0.9, 0.9, 0.9),
					},
				));
			});
	});
}

fn menu(
	mut interaction_query: Query<
		(&Interaction, &mut BackgroundColor),
		(Changed<Interaction>, With<Button>),
	>,
	mut app_state: ResMut<NextState<AppState>>,
) {
	for (interaction, mut colour) in &mut interaction_query {
		match *interaction {
			Interaction::Clicked => {
				app_state.set(AppState::GameRunning);
			},
			Interaction::Hovered => {
				*colour = BUTTON_GB_COLOUR_HOVERED.into();
			},
			Interaction::None => {
				*colour = BUTTON_BG_COLOUR.into();
			},
		}
	}
}

fn menu_cleanup(
	mut commands: Commands,
	query: Query<Entity, With<Menu>>,
) {
	for entity in query.iter() {
		commands.entity(entity).despawn_recursive();
	}
}

fn game_setup(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
) {
	// The starting y-position of the player.
	let player_y: f32 = -(HEIGHT / 2.0) + 50.0;

	// Spawn the player sprite, and scale it by 2.0
	commands.spawn((
		SpriteBundle {
			transform: Transform {
				translation: Vec3::new(0.0, player_y, 0.0),
				scale: Vec3::new(2.0, 2.0, 1.0),
				..default()
			},
			texture: asset_server.load("sprites/space_invader_player.png"),
			..default()
		},
		Player,
		Collider
	));

	// Spawn the scoreboard in the top-left
	commands.spawn(
		TextBundle::from_sections([
			TextSection::new(
				"Score: ",
				TextStyle {
					font: asset_server.load("fonts/amiga4ever/amiga4ever.ttf"),
					font_size: SCOREBOARD_FONT_SIZE,
					color: SCOREBOARD_COLOUR,
				},
			),
			TextSection::from_style(TextStyle {
				font: asset_server.load("fonts/amiga4ever/amiga4ever.ttf"),
				font_size: SCOREBOARD_FONT_SIZE,
				color: SCOREBOARD_COLOUR,
			}),
		])
		.with_style(Style {
			position_type: PositionType::Absolute,
			position: UiRect {
				top: Val::Px(5.0),
				left: Val::Px(5.0),
				..default()
			},
			..default()
		}),
	);

	// Spawn aliens at the top of the screen.

}

fn move_player(
	keyboard_input: Res<Input<KeyCode>>,

	// Get the transform properties of each Player component
	mut query: Query<&mut Transform, With<Player>>,
) {
	let mut player_transform = query.single_mut();
	let mut direction = 0.0;

	if keyboard_input.pressed(KeyCode::Left) {
		direction -= 1.0;
	}

	if keyboard_input.pressed(KeyCode::Right) {
		direction += 1.0;
	}

	let new_position = player_transform.translation.x + direction * PLAYER_SPEED * TIME_STEP;

	let left_bound = -(WIDTH / 2.0) + 32.0;
	let right_bound = (WIDTH / 2.0) - 32.0;

	player_transform.translation.x = new_position.clamp(left_bound, right_bound);
}

fn player_shoot(
	keyboard_input: Res<Input<KeyCode>>,
	mut commands: Commands,
	player_query: Query<&Transform, With<Player>>,
	mut shooting_events: EventWriter<ShootingEvent>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<ColorMaterial>>,
	mut scoreboard: ResMut<Scoreboard>,
) {
	let player_transform = player_query.single();
	let bullet_spawn_pos: Vec3 = Vec3::new(player_transform.translation.x, player_transform.translation.y, 0.0);

	if keyboard_input.just_pressed(KeyCode::Space) {
		shooting_events.send_default();

		commands.spawn((
			MaterialMesh2dBundle {
				mesh: meshes.add(shape::Quad{
					size: Vec2::new(10.0, 25.0),
					..default()
				}.into()).into(),
				material: materials.add(ColorMaterial::from(BULLET_COLOUR)),
				transform: Transform::from_translation(bullet_spawn_pos),
				..default()
			},
			Bullet,
			Velocity( Vec2::new(0.0, 1.0).normalize() * BULLET_SPEED),
		));

		scoreboard.score += 1;
	}	
}

// For bullets
fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>) {
	for (mut transform, velocity) in &mut query {
		transform.translation.y += velocity.y * TIME_STEP;
		transform.translation.x += velocity.x * TIME_STEP;
	}
}

// TODO: Implement
fn update_scoreboard(scoreboard: Res<Scoreboard>, mut query: Query<&mut Text>) {
	let mut text = query.single_mut();
	text.sections[1].value = scoreboard.score.to_string();
}

// TODO: Check if bullets hit enemies/player
fn collision_check() {

}

fn play_shooting_sound(
	mut shooting_events: EventReader<ShootingEvent>,
	audio: Res<Audio>,
	sound: Res<ShootingSound>,
) {
	if !shooting_events.is_empty() {
		shooting_events.clear();
		audio.play(sound.0.clone());
	}
}

fn remove_offscreen_entities(
	mut commands: Commands,
	query: Query<(Entity, &Transform), With<Bullet>>,
) {
	for (entity, transform) in query.iter() {
		if transform.translation.y < -HEIGHT / 2.0 {
			commands.entity(entity).despawn();
		}
	}
}