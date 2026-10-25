use macroquad::prelude::*;
use macroquad::audio::{load_sound, play_sound, stop_sound, PlaySoundParams, Sound};
use resvg::usvg::{Options, Tree};
use resvg::tiny_skia::{Pixmap, Transform};

#[derive(PartialEq)]
enum GameState {
    MainMenu,
    Playing,
    TouchTransition, // Écran noir de 3 secondes
    GameOverWait,    // Dévoilement du changement (3 secondes)
    GameOver,
    Credits,
}

struct Star {
    x: f32,
    y: f32,
    size: f32,
}

struct Obstacle {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    speed: f32,
    is_arrow: bool,
    is_znaktrapper: bool,
    is_eaten: bool,
    passed: bool,
}

fn load_svg_texture(path: &str, target_width: u32, target_height: u32) -> Option<Texture2D> {
    let svg_data = std::fs::read(path).ok()?;
    let opt = Options::default();
    let fontdb = resvg::usvg::fontdb::Database::new();
    let tree = Tree::from_data(&svg_data, &opt, &fontdb).ok()?;

    let mut pixmap = Pixmap::new(target_width, target_height)?;

    let scale_x = target_width as f32 / tree.size().width();
    let scale_y = target_height as f32 / tree.size().height();
    let transform = Transform::from_scale(scale_x, scale_y);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let image = Image {
        bytes: pixmap.data().to_vec(),
        width: target_width as u16,
        height: target_height as u16,
    };

    Some(Texture2D::from_image(&image))
}

fn draw_button(text: &str, x: f32, y: f32, width: f32, height: f32, font: Option<&Font>) -> bool {
    let mouse_pos = mouse_position();
    let rect = Rect::new(x, y, width, height);
    let hovered = rect.contains(vec2(mouse_pos.0, mouse_pos.1));

    let bg_color = if hovered { DARKBLUE } else { BLUE };
    draw_rectangle(x, y, width, height, bg_color);
    draw_rectangle_lines(x, y, width, height, 3.0, WHITE);

    let text_dim = measure_text(text, font, 22, 1.0);
    draw_text_ex(
        text,
        x + (width - text_dim.width) / 2.0,
        y + (height + text_dim.height) / 2.0,
        TextParams {
            font,
            font_size: 22,
            color: WHITE,
            ..Default::default()
        },
    );

    hovered && is_mouse_button_pressed(MouseButton::Left)
}

#[macroquad::main("Run Run my Znak")]
async fn main() {
    let font = load_ttf_font("font.otf").await.ok();

    let tex_normal = load_svg_texture("img/Normal_Character.svg", 40, 40);
    let tex_shield = load_svg_texture("img/Shield_Character.svg", 40, 40);
    let tex_game_over_char = load_svg_texture("img/GameOver_Character.svg", 40, 40);
    
    let tex_floor = load_svg_texture("img/Floor.svg", 1600, 40);

    let tex_arrow = load_svg_texture("img/Arrows.svg", 30, 15);
    let tex_log = load_svg_texture("img/Log_Trap.svg", 40, 30);
    let tex_znaktrapper_normal = load_svg_texture("img/Znaktrapper_Normal.svg", 45, 45);
    let tex_znaktrapper_eat = load_svg_texture("img/Znaktrapper_Eat.svg", 50, 50);

    let tex_castle_day = load_svg_texture("img/Castle_Background_Day.svg", 200, 150);
    let tex_castle_night = load_svg_texture("img/Castle_Background_Night.svg", 200, 150);
    let tex_star = load_svg_texture("img/Star_Background.svg", 10, 10);
    let tex_sun = load_svg_texture("img/Sun_Background.svg", 50, 50);
    let tex_moon = load_svg_texture("img/Moon_Background.svg", 45, 45);

    let tex_logo = load_texture("img/Logo.png").await.ok();

    let sfx_jump: Option<Sound> = load_sound("sfx/jump.wav").await.ok();
    let sfx_point: Option<Sound> = load_sound("sfx/point.wav").await.ok();
    let sfx_touch: Option<Sound> = load_sound("sfx/touch.wav").await.ok();
    let sfx_touch_znaktrapper: Option<Sound> = load_sound("sfx/touch_by_Znaktrapper.wav").await.ok();

    let ost_main: Option<Sound> = load_sound("music/main_ost.wav").await.ok();
    let ost_game: Option<Sound> = load_sound("music/game_ost.wav").await.ok();

    let credits_content = std::fs::read_to_string("credits.txt").unwrap_or_else(|_| "Created by Stormwindsky".to_string());
    let credits_lines: Vec<String> = credits_content.lines().map(|s| s.to_string()).collect();

    let mut state = GameState::MainMenu;
    let mut show_exit_popup = false;
    let mut credits_scroll_y = 0.0;

    let ground_y = 360.0;
    let mut player_y = ground_y - 40.0;
    let mut velocity = 0.0;
    let gravity = 1400.0;
    let jump_strength = -520.0;
    let player_x = 100.0;
    let player_size = 40.0;

    let mut score: u32 = 0;
    let mut high_score: u32 = 0;
    let mut obstacles: Vec<Obstacle> = Vec::new();
    let mut obstacle_timer = 0.0;
    let obstacle_spawn_rate = 1.8;
    let base_game_speed = 250.0;

    let mut bg_castle_x = 0.0;
    let mut floor_scroll_x = 0.0;

    let mut day_night_cycle: f32 = 0.0; 

    let mut stars: Vec<Star> = Vec::new();
    for _ in 0..30 {
        stars.push(Star {
            x: rand::gen_range(-200.0, 1200.0),
            y: rand::gen_range(20.0, 200.0),
            size: rand::gen_range(6.0, 14.0),
        });
    }

    let mut transition_timer = 0.0;
    let mut died_by_znaktrapper = false;

    if let Some(ref music) = ost_main {
        play_sound(music, PlaySoundParams { looped: true, volume: 0.8 });
    }

    loop {
        let delta = get_frame_time();
        let time = get_time();

        let sky_color = if state == GameState::MainMenu || state == GameState::Credits {
            SKYBLUE
        } else {
            day_night_cycle = (day_night_cycle + delta * 0.03) % 1.0;
            if day_night_cycle < 0.4 {
                SKYBLUE
            } else if day_night_cycle < 0.5 {
                Color::new(0.9, 0.5, 0.2, 1.0)
            } else if day_night_cycle < 0.9 {
                Color::new(0.05, 0.05, 0.15, 1.0)
            } else {
                Color::new(0.5, 0.3, 0.4, 1.0)
            }
        };

        clear_background(sky_color);

        let is_in_game_view = state == GameState::Playing || state == GameState::TouchTransition || state == GameState::GameOverWait;
        
        if is_in_game_view {
            let zoom_level = 1.5;
            let camera = Camera2D {
                target: vec2(player_x + player_size / 2.0, ground_y - 100.0),
                zoom: vec2(zoom_level / (screen_width() * 0.5), zoom_level / (screen_height() * 0.5)),
                ..Default::default()
            };
            set_camera(&camera);
        }

        // Affichage du Soleil et de la Lune selon le cycle
        if is_in_game_view {
            if day_night_cycle < 0.5 {
                // Trajectoire du Soleil (0.0 à 0.5)
                let sun_progress = day_night_cycle / 0.5;
                let sun_x = player_x - 200.0 + (sun_progress * 800.0);
                let sun_y = 180.0 - ((sun_progress * std::f32::consts::PI).sin() * 120.0);

                if let Some(ref tex) = tex_sun {
                    draw_texture_ex(tex, sun_x, sun_y, WHITE, DrawTextureParams {
                        dest_size: Some(vec2(50.0, 50.0)),
                        ..Default::default()
                    });
                }
            } else {
                // Trajectoire de la Lune (0.5 à 1.0)
                let moon_progress = (day_night_cycle - 0.5) / 0.5;
                let moon_x = player_x - 200.0 + (moon_progress * 800.0);
                let moon_y = 180.0 - ((moon_progress * std::f32::consts::PI).sin() * 120.0);

                if let Some(ref tex) = tex_moon {
                    draw_texture_ex(tex, moon_x, moon_y, WHITE, DrawTextureParams {
                        dest_size: Some(vec2(45.0, 45.0)),
                        ..Default::default()
                    });
                }
            }
        }

        if is_in_game_view && day_night_cycle >= 0.5 && day_night_cycle < 0.9 {
            for star in &stars {
                if let Some(ref tex) = tex_star {
                    draw_texture_ex(tex, star.x, star.y, WHITE, DrawTextureParams {
                        dest_size: Some(vec2(star.size, star.size)),
                        ..Default::default()
                    });
                } else {
                    draw_rectangle(star.x, star.y, star.size, star.size, YELLOW);
                }
            }
        }

        if is_in_game_view {
            if state == GameState::Playing {
                bg_castle_x -= base_game_speed * 0.3 * delta;
                if bg_castle_x <= -600.0 {
                    bg_castle_x = 0.0;
                }
            }

            let castle_alpha = if day_night_cycle >= 0.4 && day_night_cycle < 0.5 {
                1.0 - ((day_night_cycle - 0.4) * 10.0)
            } else if day_night_cycle >= 0.5 && day_night_cycle < 0.9 {
                0.0
            } else {
                1.0
            };

            for i in -1..4 {
                let castle_pos_x = bg_castle_x + (i as f32 * 300.0);
                if castle_alpha > 0.0 {
                    if let Some(ref tex) = tex_castle_day {
                        draw_texture_ex(tex, castle_pos_x, ground_y - 120.0, Color::new(1.0, 1.0, 1.0, castle_alpha), DrawTextureParams {
                            dest_size: Some(vec2(180.0, 120.0)),
                            ..Default::default()
                        });
                    }
                }

                if day_night_cycle >= 0.4 {
                    let night_alpha = if day_night_cycle >= 0.4 && day_night_cycle < 0.5 {
                        (day_night_cycle - 0.4) * 10.0
                    } else if day_night_cycle >= 0.5 && day_night_cycle < 0.9 {
                        1.0
                    } else {
                        1.0 - ((day_night_cycle - 0.9) * 10.0)
                    };

                    if let Some(ref tex) = tex_castle_night {
                        draw_texture_ex(tex, castle_pos_x, ground_y - 120.0, Color::new(1.0, 1.0, 1.0, night_alpha), DrawTextureParams {
                            dest_size: Some(vec2(180.0, 120.0)),
                            ..Default::default()
                        });
                    }
                }
            }

            if state == GameState::Playing {
                floor_scroll_x -= base_game_speed * delta;
                if floor_scroll_x <= -1600.0 {
                    floor_scroll_x = 0.0;
                }
            }
        }

        if is_in_game_view {
            let floor_width = 1600.0;
            let floor_height = 40.0;
            if let Some(ref tex) = tex_floor {
                draw_texture_ex(tex, floor_scroll_x - floor_width, ground_y, WHITE, DrawTextureParams { dest_size: Some(vec2(floor_width, floor_height)), ..Default::default() });
                draw_texture_ex(tex, floor_scroll_x, ground_y, WHITE, DrawTextureParams { dest_size: Some(vec2(floor_width, floor_height)), ..Default::default() });
                draw_texture_ex(tex, floor_scroll_x + floor_width, ground_y, WHITE, DrawTextureParams { dest_size: Some(vec2(floor_width, floor_height)), ..Default::default() });
            } else {
                draw_rectangle(-10000.0, ground_y, 20000.0, floor_height, GREEN);
            }
            draw_rectangle(-10000.0, ground_y + floor_height, 20000.0, 600.0, BROWN);
        }

        match state {
            GameState::MainMenu => {
                let center_x = screen_width() / 2.0;
                let logo_y_offset = (time * 3.0).sin() as f32 * 12.0;
                let logo_y = 50.0 + logo_y_offset;

                if let Some(ref logo) = tex_logo {
                    draw_texture_ex(
                        logo,
                        center_x - 160.0,
                        logo_y,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(320.0, 160.0)),
                            ..Default::default()
                        },
                    );
                } else {
                    let title = "Run Run my Znak";
                    let dim = measure_text(title, font.as_ref(), 50, 1.0);
                    draw_text_ex(
                        title,
                        center_x - dim.width / 2.0,
                        logo_y + 80.0,
                        TextParams { font: font.as_ref(), font_size: 50, color: DARKBLUE, ..Default::default() },
                    );
                }

                let btn_w = 220.0;
                let btn_h = 45.0;
                let start_btn_y = 240.0;

                if !show_exit_popup {
                    if draw_button("Play", center_x - btn_w / 2.0, start_btn_y, btn_w, btn_h, font.as_ref()) {
                        if let Some(ref music) = ost_main { stop_sound(music); }
                        if let Some(ref music) = ost_game { play_sound(music, PlaySoundParams { looped: true, volume: 0.8 }); }

                        player_y = ground_y - player_size;
                        velocity = 0.0;
                        score = 0;
                        obstacles.clear();
                        obstacle_timer = 0.0;
                        day_night_cycle = 0.0;
                        died_by_znaktrapper = false;
                        state = GameState::Playing;
                    }

                    if draw_button("Credits", center_x - btn_w / 2.0, start_btn_y + 60.0, btn_w, btn_h, font.as_ref()) {
                        credits_scroll_y = 0.0;
                        state = GameState::Credits;
                    }

                    if draw_button("Leave", center_x - btn_w / 2.0, start_btn_y + 120.0, btn_w, btn_h, font.as_ref()) {
                        show_exit_popup = true;
                    }
                }

                if show_exit_popup {
                    let pop_w = 380.0;
                    let pop_h = 220.0;
                    let pop_x = center_x - pop_w / 2.0;
                    let pop_y = screen_height() / 2.0 - pop_h / 2.0;

                    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.6));
                    draw_rectangle(pop_x, pop_y, pop_w, pop_h, DARKGRAY);
                    draw_rectangle_lines(pop_x, pop_y, pop_w, pop_h, 4.0, WHITE);

                    let q_text = "Are you sure you want to leave?";
                    let dim = measure_text(q_text, font.as_ref(), 20, 1.0);
                    draw_text_ex(
                        q_text,
                        pop_x + (pop_w - dim.width) / 2.0,
                        pop_y + 60.0,
                        TextParams { font: font.as_ref(), font_size: 20, color: WHITE, ..Default::default() },
                    );

                    let p_btn_w = 120.0;
                    let p_btn_h = 40.0;

                    if draw_button("Proceed", pop_x + 40.0, pop_y + 130.0, p_btn_w, p_btn_h, font.as_ref()) {
                        std::process::exit(0);
                    }

                    if draw_button("Cancel", pop_x + pop_w - 40.0 - p_btn_w, pop_y + 130.0, p_btn_w, p_btn_h, font.as_ref()) {
                        show_exit_popup = false;
                    }
                }
            }

            GameState::Credits => {
                let center_x = screen_width() / 2.0;
                let title = "CREDITS";
                let dim = measure_text(title, font.as_ref(), 40, 1.0);
                draw_text_ex(
                    title,
                    center_x - dim.width / 2.0,
                    50.0,
                    TextParams { font: font.as_ref(), font_size: 40, color: DARKBLUE, ..Default::default() },
                );

                let mouse_wheel = mouse_wheel().1;
                credits_scroll_y += mouse_wheel * 25.0;
                if is_key_down(KeyCode::Up) { credits_scroll_y += 5.0; }
                if is_key_down(KeyCode::Down) { credits_scroll_y -= 5.0; }
                if credits_scroll_y > 0.0 { credits_scroll_y = 0.0; }

                let start_y = 110.0 + credits_scroll_y;
                let mouse_pos = mouse_position();

                for (i, line) in credits_lines.iter().enumerate() {
                    let line_y = start_y + (i as f32 * 32.0);

                    if line_y > 80.0 && line_y < screen_height() - 80.0 {
                        let is_url = line.starts_with("http://") || line.starts_with("https://");
                        let text_color = if is_url { YELLOW } else { WHITE };

                        let line_dim = measure_text(line, font.as_ref(), 20, 1.0);
                        let line_x = center_x - line_dim.width / 2.0;

                        if is_url {
                            let link_rect = Rect::new(line_x, line_y - 18.0, line_dim.width, 24.0);
                            let hovered = link_rect.contains(vec2(mouse_pos.0, mouse_pos.1));

                            let draw_color = if hovered { GOLD } else { YELLOW };
                            draw_text_ex(
                                line,
                                line_x,
                                line_y,
                                TextParams { font: font.as_ref(), font_size: 20, color: draw_color, ..Default::default() },
                            );
                            draw_line(line_x, line_y + 3.0, line_x + line_dim.width, line_y + 3.0, 2.0, draw_color);

                            if hovered && is_mouse_button_pressed(MouseButton::Left) {
                                let _ = open::that(line);
                            }
                        } else {
                            draw_text_ex(
                                line,
                                line_x,
                                line_y,
                                TextParams { font: font.as_ref(), font_size: 20, color: text_color, ..Default::default() },
                            );
                        }
                    }
                }

                if draw_button("Back", 20.0, screen_height() - 60.0, 120.0, 40.0, font.as_ref()) {
                    state = GameState::MainMenu;
                }
            }

            GameState::Playing => {
                if (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Up)) && player_y >= ground_y - player_size {
                    velocity = jump_strength;
                    if let Some(ref sound) = sfx_jump { play_sound(sound, PlaySoundParams { looped: false, volume: 1.0 }); }
                }

                velocity += gravity * delta;
                player_y += velocity * delta;
                if player_y > ground_y - player_size {
                    player_y = ground_y - player_size;
                    velocity = 0.0;
                }

                obstacle_timer += delta;
                if obstacle_timer >= obstacle_spawn_rate {
                    obstacle_timer = 0.0;
                    let rand_type = rand::gen_range(0, 3);
                    
                    let spawn_x = player_x + screen_width() * 0.8;

                    if rand_type == 0 {
                        let arrow_speed = base_game_speed * rand::gen_range(1.2, 2.2);
                        obstacles.push(Obstacle {
                            x: spawn_x,
                            y: player_y + (player_size / 2.0) - 7.5,
                            width: 30.0,
                            height: 15.0,
                            speed: arrow_speed,
                            is_arrow: true,
                            is_znaktrapper: false,
                            is_eaten: false,
                            passed: false,
                        });
                    } else if rand_type == 1 {
                        obstacles.push(Obstacle {
                            x: spawn_x,
                            y: ground_y - 30.0,
                            width: 40.0,
                            height: 30.0,
                            speed: base_game_speed,
                            is_arrow: false,
                            is_znaktrapper: false,
                            is_eaten: false,
                            passed: false,
                        });
                    } else {
                        obstacles.push(Obstacle {
                            x: spawn_x,
                            y: ground_y - 45.0,
                            width: 45.0,
                            height: 45.0,
                            speed: base_game_speed,
                            is_arrow: false,
                            is_znaktrapper: true,
                            is_eaten: false,
                            passed: false,
                        });
                    }
                }

                let is_shielding = is_key_down(KeyCode::Z);
                let player_rect = Rect::new(player_x, player_y, player_size, player_size);
                let mut died = false;

                for obs in obstacles.iter_mut() {
                    obs.x -= obs.speed * delta;
                    let obs_rect = Rect::new(obs.x, obs.y, obs.width, obs.height);

                    if player_rect.overlaps(&obs_rect) {
                        if obs.is_arrow && is_shielding {
                            obs.x = -1000.0;
                        } else if obs.is_znaktrapper {
                            obs.is_eaten = true;
                            died = true;
                            died_by_znaktrapper = true;
                        } else {
                            died = true;
                            if let Some(ref sound) = sfx_touch { play_sound(sound, PlaySoundParams { looped: false, volume: 1.0 }); }
                        }
                    }

                    if !obs.passed && obs.x + obs.width < player_x {
                        obs.passed = true;
                        score += 1;
                        if score > high_score { high_score = score; }
                        if let Some(ref sound) = sfx_point { play_sound(sound, PlaySoundParams { looped: false, volume: 1.0 }); }
                    }
                }

                obstacles.retain(|o| o.x + o.width > player_x - 300.0);

                if died {
                    if let Some(ref music) = ost_game { stop_sound(music); }
                    if died_by_znaktrapper {
                        if let Some(ref sound) = sfx_touch_znaktrapper { play_sound(sound, PlaySoundParams { looped: false, volume: 1.0 }); }
                        state = GameState::TouchTransition;
                    } else {
                        state = GameState::GameOver;
                    }
                    transition_timer = 0.0;
                }

                for obs in &obstacles {
                    if obs.is_arrow {
                        if let Some(ref tex) = tex_arrow {
                            draw_texture_ex(tex, obs.x, obs.y, WHITE, DrawTextureParams { dest_size: Some(vec2(obs.width, obs.height)), ..Default::default() });
                        }
                    } else if obs.is_znaktrapper {
                        let tex_to_use = if obs.is_eaten { &tex_znaktrapper_eat } else { &tex_znaktrapper_normal };
                        if let Some(ref tex) = tex_to_use {
                            draw_texture_ex(tex, obs.x, obs.y, WHITE, DrawTextureParams { dest_size: Some(vec2(obs.width, obs.height)), ..Default::default() });
                        }
                    } else {
                        if let Some(ref tex) = tex_log {
                            draw_texture_ex(tex, obs.x, obs.y, WHITE, DrawTextureParams { dest_size: Some(vec2(obs.width, obs.height)), ..Default::default() });
                        }
                    }
                }

                if is_shielding {
                    if let Some(ref tex) = tex_shield {
                        draw_texture_ex(tex, player_x, player_y, WHITE, DrawTextureParams { dest_size: Some(vec2(player_size, player_size)), ..Default::default() });
                    }
                } else {
                    if let Some(ref tex) = tex_normal {
                        draw_texture_ex(tex, player_x, player_y, WHITE, DrawTextureParams { dest_size: Some(vec2(player_size, player_size)), ..Default::default() });
                    }
                }

                set_default_camera();
                let score_str = format!("Score: {}", score);
                draw_text_ex(&score_str, 20.0, 40.0, TextParams { font: font.as_ref(), font_size: 30, color: WHITE, ..Default::default() });
            }

            GameState::TouchTransition => {
                transition_timer += delta;
                
                clear_background(BLACK);

                for obs in &obstacles {
                    if obs.is_znaktrapper && obs.is_eaten {
                        if let Some(ref tex) = tex_znaktrapper_eat {
                            draw_texture_ex(tex, obs.x, obs.y, WHITE, DrawTextureParams {
                                dest_size: Some(vec2(obs.width, obs.height)),
                                ..Default::default()
                            });
                        }
                    }
                }

                if transition_timer >= 3.0 {
                    state = GameState::GameOverWait;
                    transition_timer = 0.0;
                }
            }

            GameState::GameOverWait => {
                transition_timer += delta;

                for obs in &obstacles {
                    if obs.is_arrow {
                        if let Some(ref tex) = tex_arrow {
                            draw_texture_ex(tex, obs.x, obs.y, WHITE, DrawTextureParams { dest_size: Some(vec2(obs.width, obs.height)), ..Default::default() });
                        }
                    } else if obs.is_znaktrapper {
                        let tex_to_use = if obs.is_eaten { &tex_znaktrapper_eat } else { &tex_znaktrapper_normal };
                        if let Some(ref tex) = tex_to_use {
                            draw_texture_ex(tex, obs.x, obs.y, WHITE, DrawTextureParams { dest_size: Some(vec2(obs.width, obs.height)), ..Default::default() });
                        }
                    } else {
                        if let Some(ref tex) = tex_log {
                            draw_texture_ex(tex, obs.x, obs.y, WHITE, DrawTextureParams { dest_size: Some(vec2(obs.width, obs.height)), ..Default::default() });
                        }
                    }
                }

                if !died_by_znaktrapper {
                    if let Some(ref tex) = tex_game_over_char {
                        draw_texture_ex(tex, player_x, player_y, WHITE, DrawTextureParams { dest_size: Some(vec2(player_size, player_size)), ..Default::default() });
                    }
                }

                if transition_timer >= 3.0 {
                    state = GameState::GameOver;
                }
            }

            GameState::GameOver => {
                set_default_camera();
                draw_text_ex("GAME OVER", screen_width() / 2.0 - 110.0, screen_height() / 3.0, TextParams { font: font.as_ref(), font_size: 50, color: RED, ..Default::default() });

                let final_score_str = format!("Score: {}  |  Best: {}", score, high_score);
                draw_text_ex(&final_score_str, screen_width() / 2.0 - 130.0, screen_height() / 2.0, TextParams { font: font.as_ref(), font_size: 30, color: WHITE, ..Default::default() });

                draw_text_ex("Press SPACE to return to Menu", screen_width() / 2.0 - 160.0, screen_height() / 2.0 + 60.0, TextParams { font: font.as_ref(), font_size: 25, color: LIGHTGRAY, ..Default::default() });

                if is_key_pressed(KeyCode::Space) {
                    if let Some(ref music) = ost_main { play_sound(music, PlaySoundParams { looped: true, volume: 0.8 }); }
                    state = GameState::MainMenu;
                }
            }
        }

        next_frame().await
    }
}
