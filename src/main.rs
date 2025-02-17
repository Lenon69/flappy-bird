use std::f32::consts::PI;

use bevy::math::bounding::{Aabb2d, IntersectsVolume};
use bevy::{prelude::*, window::WindowResolution};
use rand::Rng;

const NORMAL_BUTTON: Color = Color::srgb(0.34, 0.34, 0.34);
const HOVERED_BUTTON: Color = Color::srgb(0.44, 0.44, 0.44);
const PRESSED_BUTTON: Color = Color::srgb(0.24, 0.24, 0.24);

// Komponenty pozycji, prędkości, czasu życia, rozmiaru oraz znacznik gracza
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
enum AppState {
    #[default]
    Menu,
    Playing,
    GameOver,
}

#[derive(Component)]
struct Velocity {
    dx: f32,
    dy: f32,
}

#[derive(Component)]
struct LifeTime(f32);

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Pipe;

#[derive(Resource)]
struct Gravity(f32);

#[derive(Component)]
struct Collider {
    half_size: Vec2,
}

#[derive(Component)]
struct Scoreable {
    passed: bool,
}

#[derive(Resource, Default)]
struct Score(i32);

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct Menu;

#[derive(Component)]
struct StartButton;

#[derive(Component)]
struct ExitButton;

#[derive(Component)]
struct GameOverUI;

#[derive(Component)]
struct RestartButton;

//
// SYSTEMY
//

// System ruchu: aktualizuje Transform na podstawie Velocity
fn move_system(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, vel) in query.iter_mut() {
        transform.translation.x += vel.dx * time.delta_secs();
        transform.translation.y += vel.dy * time.delta_secs();
    }
}

// System obsługi wejścia – dla gracza.
// Używamy Res<Input<KeyCode>> (typowo w Bevy) do sprawdzania przycisków.
fn player_input_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Velocity, With<Player>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        for mut vel in query.iter_mut() {
            vel.dy = 150.0;
        }
    }
}

// System obsługi czasu życia – zmniejsza LifeTime o upływ czasu i usuwa encję, gdy czas osiągnie 0.
fn lifetime_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut LifeTime)>,
    time: Res<Time>,
) {
    for (entity, mut lifetime) in query.iter_mut() {
        lifetime.0 -= time.delta_secs();
        if lifetime.0 <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

// System generowania przeszkód (rur).
fn spawn_pipes(mut commands: Commands, asset_server: Res<AssetServer>) {
    let gap = 100.0;
    let pipe_speed = -100.0;
    let pipe_size = Vec2::new(50.0, 600.0);

    let mut rng = rand::thread_rng();
    let center_y = rng.gen_range(-130.0..=130.0);

    // Obliczamy pozycje dla rur:
    let top_pipe_y = center_y + gap / 2.0 + pipe_size.y / 2.0;
    let bottom_pipe_y = center_y - gap / 2.0 - pipe_size.y / 2.0;

    // Górna rura
    commands.spawn((
        Sprite {
            image: asset_server.load(r"sprites\pipe-green.png"),
            custom_size: Some(pipe_size),
            ..Default::default()
        },
        Velocity {
            dx: pipe_speed,
            dy: 0.0,
        },
        Collider {
            half_size: pipe_size / 2.0 - 5.0,
        },
        Pipe,
        LifeTime(10.0),
        Scoreable { passed: false },
        Transform {
            translation: Vec3::new(500.0, top_pipe_y, 0.0),
            rotation: Quat::from_rotation_x(PI),
            ..Default::default()
        }, // Transform::from_xyz(400.0, top_pipe_y, 0.0),
    ));

    // Dolna rura
    commands.spawn((
        Sprite {
            image: asset_server.load(r"sprites\pipe-green.png"),
            custom_size: Some(pipe_size),
            ..Default::default()
        },
        Velocity {
            dx: pipe_speed,
            dy: 0.0,
        },
        Collider {
            half_size: pipe_size / 2.0 - 5.0,
        },
        Pipe,
        LifeTime(10.0),
        Transform::from_xyz(500.0, bottom_pipe_y, 0.0),
    ));
}

fn pipe_spawn_system(
    commands: Commands,
    time: Res<Time>,
    mut timer: Local<Timer>,
    asset_server: Res<AssetServer>,
) {
    if timer.duration().as_secs_f32() == 0.0 {
        *timer = Timer::from_seconds(2.0, TimerMode::Repeating)
    }

    if timer.tick(time.delta()).just_finished() {
        spawn_pipes(commands, asset_server);
    }
}

fn score_system(
    mut score: ResMut<Score>,
    player_query: Query<&Transform, With<Player>>,
    mut pipe_query: Query<(&Transform, &mut Scoreable), With<Pipe>>,
) {
    let Ok(player_transfomr) = player_query.get_single() else {
        return;
    };

    for (pipe_transform, mut scoreable) in pipe_query.iter_mut() {
        if !scoreable.passed && player_transfomr.translation.x > pipe_transform.translation.x {
            score.0 += 1;
            scoreable.passed = true;
        }
    }
}

// System wykrywający kolizje – sprawdza pary encji i przy kolizji zmienia kolor sprite’a.
fn collision_system(
    player_query: Query<(Entity, &Transform), (With<Player>, Without<Pipe>)>,
    pipe_query: Query<(Entity, &Transform), With<Pipe>>,
    collider_query: Query<&Collider>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let Ok((player_entity, player_transform)) = player_query.get_single() else {
        return;
    };

    let Ok(player_collider) = collider_query.get(player_entity) else {
        return;
    };

    let player_aabb = Aabb2d::new(
        player_transform.translation.truncate(),
        player_collider.half_size,
    );

    for (pipe_entity, pipe_transform) in pipe_query.iter() {
        let Ok(pipe_collider) = collider_query.get(pipe_entity) else {
            continue;
        };

        let pipe_aabb = Aabb2d::new(
            pipe_transform.translation.truncate(),
            pipe_collider.half_size,
        );

        if player_aabb.intersects(&pipe_aabb) {
            next_state.set(AppState::GameOver);
            return;
        }
    }
}

fn boundary_collision_system(
    player_query: Query<(&Transform, &Collider), With<Player>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let Ok((transform, collider)) = player_query.get_single() else {
        return;
    };

    // Granice ekranu (dla rozdzielczości 800x600)
    let top_boundary = 300.0; // 600/2 = 300
    let bottom_boundary = -300.0;

    // Oblicz pozycje krańców gracza
    let player_top = transform.translation.y + collider.half_size.y;
    let player_bottom = transform.translation.y - collider.half_size.y;

    // Sprawdź kolizje z granicami
    if player_top > top_boundary || player_bottom < bottom_boundary {
        next_state.set(AppState::GameOver);
    }
}

fn gravity_system(
    time: Res<Time>,
    mut query: Query<&mut Velocity, With<Player>>,
    gravity: Res<Gravity>,
) {
    let delta = time.delta_secs();
    for mut velocity in &mut query {
        velocity.dy += gravity.0 * delta;
    }
}

fn update_score_display(score: Res<Score>, mut query: Query<&mut Text2d, With<ScoreText>>) {
    for mut text in query.iter_mut() {
        text.0 = format!("Score: {}", score.0);
    }
}

fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
            Option<&StartButton>,
            Option<&ExitButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut next_state: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, mut bg_color, mut border_color, children, start_button, exit_button) in
        &mut interaction_query
    {
        // Załóżmy, że pierwszy element Children to tekst
        let mut text = text_query.get_mut(children[0]).unwrap();

        match *interaction {
            Interaction::Pressed => {
                *bg_color = PRESSED_BUTTON.into();
                border_color.0 = Color::srgb(1.0, 0.0, 0.0);
                if start_button.is_some() {
                    next_state.set(AppState::Playing);
                } else if exit_button.is_some() {
                    exit.send(AppExit::Success);
                }
            }
            Interaction::Hovered => {
                *bg_color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *bg_color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
                *text = if start_button.is_some() {
                    Text::new("Start Game")
                } else {
                    Text::new("Exit")
                };
            }
        }
    }
}

fn despawn_menu(
    mut commands: Commands,
    menu_query: Query<Entity, With<Menu>>,
    state: Res<State<AppState>>,
) {
    if *state.get() == AppState::Playing {
        for menu_entity in menu_query.iter() {
            commands.entity(menu_entity).despawn_recursive();
        }
    }
}

fn spawn_game_over_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..Default::default()
            },
            // Overlay z lekką przezroczystością
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            GameOverUI,
        ))
        .with_children(|parent| {
            // Tekst "Game Over"
            parent.spawn((
                Text::new("Game Over"),
                TextColor(Color::srgb(0.151, 0.1, 0.44)),
                TextFont {
                    font_size: 60.0,
                    ..Default::default()
                },
            ));
            // Przycisk "Restart"
            parent
                .spawn((
                    Button,
                    Interaction::default(),
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(80.0),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    BorderColor(Color::BLACK),
                    BorderRadius::MAX,
                    RestartButton,
                ))
                .with_child((
                    Text::new("Restart"),
                    TextColor(Color::WHITE),
                    TextFont {
                        font_size: 33.0,
                        ..Default::default()
                    },
                ));
            parent
                .spawn((
                    Button,
                    Interaction::default(),
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(80.0),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    BorderColor(Color::BLACK),
                    BorderRadius::MAX,
                    ExitButton,
                ))
                .with_child((
                    Text::new("Exit"),
                    TextColor(Color::WHITE),
                    TextFont {
                        font_size: 33.0,
                        ..Default::default()
                    },
                ));
        });
}

fn on_enter_game_over(mut commands: Commands) {
    spawn_game_over_ui(commands);
}

fn game_over_exit_button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &Children),
        (With<Button>, With<ExitButton>),
    >,
    mut text_query: Query<&mut Text>,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, mut bg_color, children) in interaction_query.iter_mut() {
        // Pobieramy tekst przycisku (zakładamy, że jest pierwszym dzieckiem)
        let text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Pressed => {
                *bg_color = PRESSED_BUTTON.into();
                // Wyjście z gry
                exit.send(AppExit::Success);
            }
            Interaction::Hovered => {
                *bg_color = HOVERED_BUTTON.into();
            }
            Interaction::None => {
                *bg_color = NORMAL_BUTTON.into();
            }
        }
    }
}
fn game_over_button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &Children),
        (With<Button>, With<RestartButton>),
    >,
    mut text_query: Query<&mut Text>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut bg_color, children) in &mut interaction_query {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Pressed => {
                *bg_color = PRESSED_BUTTON.into();
                next_state.set(AppState::Playing);
            }
            Interaction::Hovered => {
                *bg_color = HOVERED_BUTTON.into();
            }
            Interaction::None => {
                *bg_color = NORMAL_BUTTON.into();
            }
        }
    }
}

fn despawn_game_over_ui(
    mut commands: Commands,
    game_over_query: Query<Entity, With<GameOverUI>>,
    state: Res<State<AppState>>,
) {
    if *state.get() == AppState::Playing {
        for entity in game_over_query.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn setup_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.20, 0.20, 0.20)),
            Menu,
        ))
        .with_children(|parent| {
            // Przycisk "Start Game"
            parent
                .spawn((
                    Button,
                    Interaction::default(),
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(80.0),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    BorderColor(Color::BLACK),
                    BorderRadius::MAX,
                    StartButton,
                ))
                .with_child((Text::new("Start Game"), TextColor(Color::WHITE)));
            // Przycisk "Exit"
            parent
                .spawn((
                    Button,
                    Interaction::default(),
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(80.0),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.34, 0.34, 0.34)),
                    BorderColor(Color::BLACK),
                    BorderRadius::MAX,
                    ExitButton,
                ))
                .with_child((Text::new("Exit"), TextColor(Color::WHITE)));
        });
}

fn restart_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    // Pobieramy encje, które chcemy usunąć: gracz, rury oraz wynik
    game_query: Query<Entity, Or<(With<Player>, With<Pipe>, With<ScoreText>)>>,
) {
    // Sprzątnij poprzednią rozgrywkę.
    for entity in game_query.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Zresetuj wynik
    commands.insert_resource(Score(0));

    // Wynik
    commands.spawn((
        Text2d::new("Score: 0"),
        Transform::from_xyz(0.0, 250.0, 10.0),
        ScoreText,
    ));

    // Tło
    commands.spawn((
        Sprite {
            image: asset_server.load("sprites/background-day.png"),
            custom_size: Some(Vec2::new(800.0, 600.0)),
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.0, -1.0),
        GlobalTransform::default(),
    ));

    // Gracz
    commands.spawn((
        Sprite {
            image: asset_server.load("sprites/bluebird-midflap.png"),
            ..Default::default()
        },
        Velocity { dx: 0.0, dy: 0.0 },
        Collider {
            half_size: Vec2::new(16.0, 16.0),
        },
        Player,
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));
}

// System inicjalizacyjny – spawn gracza z komponentem Player oraz sprite’em.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    // mut next_state: ResMut<NextState<AppState>>,
) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text2d::new("Score: 0"),
        Transform::from_xyz(0.0, 250.0, 10.0),
        ScoreText,
    ));

    //Background
    commands.spawn((
        Sprite {
            image: asset_server.load(r"sprites\background-day.png"),
            custom_size: Some(Vec2::new(800.0, 600.0)),
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.0, -1.0),
        GlobalTransform::default(),
    ));

    // Player
    commands.spawn((
        Sprite {
            image: asset_server.load(r"sprites\bluebird-midflap.png"),
            ..Default::default()
        },
        Velocity { dx: 0.0, dy: 0.0 },
        Collider {
            half_size: Vec2::new(16.0, 16.0),
        },
        Player,
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));

    // next_state.set(AppState::Playing);
}

//
// MAIN
//

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Flappy Bird".to_string(),
                resolution: WindowResolution::new(800.0, 600.0),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .init_state::<AppState>()
        .insert_resource(Gravity(-350.0))
        .insert_resource(Score(0))
        .add_systems(Startup, (setup, setup_menu))
        .add_systems(Update, button_system.run_if(in_state(AppState::Menu)))
        .add_systems(
            Update,
            (
                move_system,
                gravity_system,
                collision_system,
                player_input_system,
                lifetime_system,
                pipe_spawn_system,
                boundary_collision_system,
                score_system,
                update_score_display,
                despawn_menu,
                despawn_game_over_ui,
            )
                .run_if(in_state(AppState::Playing)),
        )
        .add_systems(OnEnter(AppState::GameOver), on_enter_game_over)
        .add_systems(
            Update,
            game_over_button_system.run_if(in_state(AppState::GameOver)),
        )
        .add_systems(
            Update,
            game_over_exit_button_system.run_if(in_state(AppState::GameOver)),
        )
        .add_systems(OnEnter(AppState::Playing), restart_game)
        .run();
}
