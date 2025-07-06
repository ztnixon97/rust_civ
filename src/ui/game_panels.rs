use bevy::prelude::*;
use crate::game::units::{Unit, UnitSelection};
use crate::game::cities::City;
use crate::game::civilization::CivilizationManager;
use crate::game::game_initialization::{GameState, GamePhase};

#[derive(Component)]
pub struct GameStatusPanel;

#[derive(Component)]
pub struct UnitStatusPanel;

#[derive(Component)]
pub struct HotkeysPanel;

#[derive(Component)]
pub struct SelectedUnitInfo;

#[derive(Resource)]
pub struct UIState {
    pub show_hotkeys: bool,
    pub show_unit_status: bool,
    pub show_game_status: bool,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            show_hotkeys: true,
            show_unit_status: true,
            show_game_status: true,
        }
    }
}

// System to setup improved UI panels
pub fn setup_ui_panels(mut commands: Commands) {
    // Game Status Panel (bottom right)
    commands.spawn((
        GameStatusPanel,
        Text::new(""),
        TextLayout::new_with_justify(JustifyText::Right),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            right: Val::Px(10.0),
            width: Val::Px(250.0),
            ..default()
        },
    ));
    
    // Selected Unit Info Panel (bottom center)
    commands.spawn((
        SelectedUnitInfo,
        Text::new(""),
        TextLayout::new_with_justify(JustifyText::Center),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-200.0)), // Center the 400px wide panel
            width: Val::Px(400.0),
            ..default()
        },
    ));
    
    // Hotkeys Panel (left side)
    commands.spawn((
        HotkeysPanel,
        Text::new(""),
        TextLayout::new_with_justify(JustifyText::Left),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(100.0),
            left: Val::Px(10.0),
            width: Val::Px(200.0),
            ..default()
        },
    ));
}

// System to update game status panel
pub fn update_game_status_panel(
    mut status_query: Query<&mut Text, With<GameStatusPanel>>,
    civ_manager: Res<CivilizationManager>,
    game_state: Res<GameState>,
    unit_query: Query<&Unit>,
    city_query: Query<&City>,
    ui_state: Res<UIState>,
) {
    if !ui_state.show_game_status || !game_state.is_initialized {
        return;
    }
    
    let Ok(mut text) = status_query.single_mut() else { return };
    
    let current_civ_name = civ_manager.get_civilization(civ_manager.current_turn_civ)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    
    let phase_text = match game_state.current_phase {
        GamePhase::PlayerTurn => "Your Turn",
        GamePhase::AITurn(_) => "AI Turn",
        _ => "Processing",
    };
    
    // Count units and cities by civilization
    let mut civ_stats = std::collections::HashMap::new();
    
    for unit in unit_query.iter() {
        let entry = civ_stats.entry(unit.civilization_id).or_insert((0, 0, 0.0)); // (units, cities, military_power)
        entry.0 += 1;
        entry.2 += unit.get_combat_strength(true) as f32;
    }
    
    for city in city_query.iter() {
        let entry = civ_stats.entry(city.civilization_id).or_insert((0, 0, 0.0));
        entry.1 += 1;
    }
    
    let mut status_text = format!(
        "=== GAME STATUS ===\nTurn: {}\nActive: {} ({})\n\n",
        game_state.game_turn,
        current_civ_name,
        phase_text
    );
    
    // Show civilization summary
    status_text.push_str("Civilizations:\n");
    for (civ_id, civ) in &civ_manager.civilizations {
        let (units, cities, military) = civ_stats.get(civ_id).unwrap_or(&(0, 0, 0.0));
        let status_indicator = if *civ_id == civ_manager.current_turn_civ { "►" } else { " " };
        
        status_text.push_str(&format!(
            "{} {}: {}C/{}U/{:.0}M\n",
            status_indicator,
            civ.name.chars().take(8).collect::<String>(), // Abbreviated name
            cities,
            units,
            military
        ));
    }
    
    **text = status_text;
}

// System to update selected unit info
pub fn update_selected_unit_info(
    mut info_query: Query<&mut Text, With<SelectedUnitInfo>>,
    unit_query: Query<&Unit>,
    unit_selection: Res<UnitSelection>,
    civ_manager: Res<CivilizationManager>,
    ui_state: Res<UIState>,
    game_state: Res<GameState>,
) {
    if !ui_state.show_unit_status || !game_state.is_initialized {
        return;
    }
    
    let Ok(mut text) = info_query.single_mut() else { return };
    
    if let Some(selected_entity) = unit_selection.selected_unit {
        if let Ok(unit) = unit_query.get(selected_entity) {
            let civ_name = civ_manager.get_civilization(unit.civilization_id)
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown");
            
            let health_bar = create_health_bar(unit.health, unit.max_health);
            let experience_info = format!("XP: {}/10", unit.combat_experience % 10);
            
            let mut unit_info = format!(
                "=== SELECTED UNIT ===\n{} ({}) at ({}, {})\n{} | MP: {}/{} | {}\n",
                unit.unit_type.get_name(),
                civ_name,
                unit.hex_coord.q,
                unit.hex_coord.r,
                health_bar,
                unit.movement_points,
                unit.max_movement_points,
                experience_info
            );
            
            // Add combat stats if it's a military unit
            if unit.can_attack {
                unit_info.push_str(&format!(
                    "Combat: ATK {} | DEF {} | {} EXP\n",
                    unit.attack_strength,
                    unit.defense_strength,
                    unit.combat_experience
                ));
                
                if unit.is_fortified {
                    unit_info.push_str(&format!("FORTIFIED (Turn {})\n", unit.fortification_turns));
                }
            }
            
            // Add available actions
            let mut actions = Vec::new();
            if unit.movement_points > 0 && !unit.has_moved {
                actions.push("Move");
            }
            if unit.can_attack && unit.movement_points > 0 && !unit.has_attacked {
                actions.push("Attack (A)");
            }
            if unit.can_found_cities && unit.movement_points > 0 {
                actions.push("Found City (F)");
            }
            if unit.can_build_improvements && unit.movement_points > 0 {
                actions.push("Build (B)");
            }
            if unit.can_attack {
                actions.push("Fortify (Shift+F)");
            }
            actions.push("Skip (S)");
            
            if !actions.is_empty() {
                unit_info.push_str("Actions: ");
                unit_info.push_str(&actions.join(" | "));
                unit_info.push('\n');
            }
            
            **text = unit_info;
        } else {
            **text = "".to_string();
        }
    } else {
        **text = "No unit selected\nClick on a unit to select it".to_string();
    }
}

// System to update hotkeys panel
pub fn update_hotkeys_panel(
    mut hotkeys_query: Query<&mut Text, With<HotkeysPanel>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UIState>,
    game_state: Res<GameState>,
) {
    if !game_state.is_initialized {
        return;
    }
    
    // Toggle hotkeys panel with H
    if keyboard.just_pressed(KeyCode::KeyH) {
        ui_state.show_hotkeys = !ui_state.show_hotkeys;
    }
    
    let Ok(mut text) = hotkeys_query.single_mut() else { return };
    
    if ui_state.show_hotkeys {
        **text = format!(
            "=== HOTKEYS ===\n\
            H - Toggle this help\n\
            Click - Select/Move\n\
            SPACE - End Turn\n\
            N - Next Unit\n\
            \n\
            === UNIT ACTIONS ===\n\
            A - Attack\n\
            F - Found City\n\
            B - Build Improvement\n\
            S - Skip Unit\n\
            Shift+F - Fortify\n\
            \n\
            === VIEW ===\n\
            WASD - Move Camera\n\
            Wheel - Zoom\n\
            G - Toggle Grid\n\
            E - Toggle Elevation\n\
            Tab - Info Modes\n\
            F3 - Debug Info\n\
            \n\
            ESC - Quit Game"
        );
    } else {
        **text = "Press H for help".to_string();
    }
}

// Helper function to create a visual health bar
fn create_health_bar(current: u32, max: u32) -> String {
    let health_percent = current as f32 / max as f32;
    let bar_length = 10;
    let filled_bars = (health_percent * bar_length as f32) as usize;
    
    let mut health_bar = String::new();
    health_bar.push_str("HP:");
    
    for i in 0..bar_length {
        if i < filled_bars {
            health_bar.push('█');
        } else {
            health_bar.push('░');
        }
    }
    
    health_bar.push_str(&format!(" {}/{}", current, max));
    health_bar
}

// System to toggle UI panels
pub fn toggle_ui_panels(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UIState>,
) {
    // Toggle game status with F1
    if keyboard.just_pressed(KeyCode::F1) {
        ui_state.show_game_status = !ui_state.show_game_status;
        println!("Game status panel: {}", if ui_state.show_game_status { "ON" } else { "OFF" });
    }
    
    // Toggle unit status with F2
    if keyboard.just_pressed(KeyCode::F2) {
        ui_state.show_unit_status = !ui_state.show_unit_status;
        println!("Unit status panel: {}", if ui_state.show_unit_status { "ON" } else { "OFF" });
    }
}

// System to provide turn summary
pub fn turn_summary_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    unit_query: Query<&Unit>,
    city_query: Query<&City>,
    civ_manager: Res<CivilizationManager>,
    game_state: Res<GameState>,
) {
    if !game_state.is_initialized {
        return;
    }
    
    // Show turn summary with F4
    if keyboard.just_pressed(KeyCode::F4) {
        if let Some(current_civ) = civ_manager.get_civilization(civ_manager.current_turn_civ) {
            println!("=== TURN SUMMARY FOR {} ===", current_civ.name);
            
            // Count units by type
            let mut unit_counts = std::collections::HashMap::new();
            let mut units_that_can_move = 0;
            
            for unit in unit_query.iter() {
                if unit.civilization_id == current_civ.id {
                    *unit_counts.entry(unit.unit_type).or_insert(0) += 1;
                    if unit.movement_points > 0 && !unit.has_moved {
                        units_that_can_move += 1;
                    }
                }
            }
            
            println!("Units:");
            for (unit_type, count) in unit_counts {
                println!("  {}: {}", unit_type.get_name(), count);
            }
            
            println!("Units that can still move: {}", units_that_can_move);
            
            // City information
            let mut total_population = 0;
            let mut total_production = 0.0;
            let mut total_science = 0.0;
            let mut total_gold = 0.0;
            
            for city in city_query.iter() {
                if city.civilization_id == current_civ.id {
                    total_population += city.population;
                    total_production += city.production_per_turn;
                    total_science += city.science_per_turn;
                    total_gold += city.gold_per_turn;
                }
            }
            
            println!("Cities: {}", current_civ.cities.len());
            println!("Total Population: {}", total_population);
            println!("Per Turn: Production {:.1}, Science {:.1}, Gold {:.1}", 
                     total_production, total_science, total_gold);
            println!("Accumulated: Science {:.0}, Gold {:.0}", 
                     current_civ.science_points, current_civ.gold);
        }
    }
}