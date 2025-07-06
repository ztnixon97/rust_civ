mod game;
mod ui;

use bevy::prelude::*;
use game::*;
use game::camera_zoom::camera_zoom_system;
use game::map::{get_climate_description, evaluate_tile_suitability, toggle_elevation_shading, adjust_elevation_intensity};
use game::world_gen::StrategicFeature;
use game::civilization::CivilizationManager;
use game::units::{UnitSelection, unit_selection_system, start_unit_turns, spawn_unit_markers, update_unit_marker_positions};
use game::cities::{process_city_turns, spawn_city_markers};
use game::game_initialization::{GameState, initialize_game, turn_system, ai_turn_system, display_turn_info, setup_turn_info_ui, TurnInfoText};
use game::city_founding::{CityFoundingState, city_founding_system, worker_actions_system, skip_unit_system, fortify_system, auto_turn_advance_system};
use game::combat::{CombatState, combat_system, cleanup_dead_units_system};
use ui::game_panels::{UIState, setup_ui_panels, update_game_status_panel, update_selected_unit_info, update_hotkeys_panel, toggle_ui_panels, turn_summary_system};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Civilization Game - Realistic World".into(),
                resolution: (1400.0, 800.0).into(),
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(CullingPlugin)
        .insert_resource(GridSettings::default())
        .insert_resource(HoverState::default())
        .insert_resource(InfoDisplayMode::Basic)
        .insert_resource(CivilizationManager::default())
        .insert_resource(UnitSelection::default())
        .insert_resource(GameState::default())
        .insert_resource(CityFoundingState::default())
        .insert_resource(CombatState::default())
        .insert_resource(UIState::default())
        .add_systems(Startup, (
            setup, 
            setup_map, 
            setup_grid_lines,
            setup_turn_info_ui,
            setup_ui_panels,
        ))
        // Alternative world types (uncomment one to try):
        // .add_systems(Startup, (setup, setup_pangaea_world, setup_grid_lines, setup_turn_info_ui))
        // .add_systems(Startup, (setup, setup_archipelago_world, setup_grid_lines, setup_turn_info_ui))
        // .add_systems(Startup, (setup, setup_fragmented_world, setup_grid_lines, setup_turn_info_ui))
        // .add_systems(Startup, (setup, setup_dual_supercontinents, setup_grid_lines, setup_turn_info_ui))
        // .add_systems(Startup, (setup, setup_mediterranean_world, setup_grid_lines, setup_turn_info_ui))
        .add_systems(Update, (
            // Core game systems (Group 1)
            initialize_game,
            turn_system,
            ai_turn_system,
            display_turn_info,
            process_city_turns,
            start_unit_turns,
            cleanup_dead_units_system,
        ))
        .add_systems(Update, (
            // Player actions (Group 2)
            city_founding_system,
            worker_actions_system,
            skip_unit_system,
            fortify_system,
            auto_turn_advance_system,
            combat_system,
        ))
        .add_systems(Update, (
            // Visual and UI systems (Group 3)
            spawn_city_markers,
            spawn_unit_markers,
            update_unit_marker_positions,
            update_game_status_panel,
            update_selected_unit_info,
            update_hotkeys_panel,
            toggle_ui_panels,
            turn_summary_system,
        ))
        .add_systems(Update, (
            // Input and interaction (Group 4)
            camera_movement, 
            camera_zoom_system,
            basic_input, 
            hex_hover_system,
            unit_selection_system,
        ))
        .add_systems(Update, (
            // Debug and world info (Group 5)
            debug_info_system,
            toggle_grid_system,
            spawn_resource_markers,
            tile_info_system,
            toggle_info_display,
            toggle_elevation_shading_system,
            adjust_elevation_intensity_system,
        ))
        .run();
}

#[derive(Component)]
struct TileInfoText;

#[derive(Component)]
struct WorldStatsText;

#[derive(Resource, Default)]
struct HoverState {
    current_hovered: Option<HexCoord>,
    previous_hovered: Option<HexCoord>,
}

#[derive(Resource)]
enum InfoDisplayMode {
    Basic,
    Climate,
    Resources,
    Suitability,
    Strategic,
}

impl Default for InfoDisplayMode {
    fn default() -> Self {
        InfoDisplayMode::Basic
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    
    // Main controls text - updated with new controls
    commands.spawn((
        Text::new("Civ Game - WASD:Camera, Wheel:Zoom, G:Grid, H:Help, F1-F4:UI Panels, SPACE:End Turn, Click:Select/Move, A:Attack, F:Found City, B:Build, S:Skip, ESC:Quit"),
        TextLayout::new_with_justify(JustifyText::Left),
        TextFont {
            font_size: 12.0, // Smaller to fit more text
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            width: Val::Px(1200.0), // Even wider to accommodate all the new controls
            ..default()
        },
    ));
    
    // World stats display (top right)
    commands.spawn((
        WorldStatsText,
        Text::new(""),
        TextLayout::new_with_justify(JustifyText::Right),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(60.0), // Moved down to make room for turn info
            right: Val::Px(10.0),
            ..default()
        },
    ));
    
    // Detailed tile info display (bottom left)
    commands.spawn((
        TileInfoText,
        Text::new(""),
        TextLayout::new_with_justify(JustifyText::Left),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            width: Val::Px(350.0),
            ..default()
        },
    ));
}

fn tile_info_system(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    tile_query: Query<&MapTile, Without<Culled>>,
    unit_query: Query<&game::units::Unit>,
    city_query: Query<&game::cities::City>,
    mut info_text_query: Query<&mut Text, (With<TileInfoText>, Without<WorldStatsText>, Without<TurnInfoText>)>,
    mut world_stats_query: Query<&mut Text, (With<WorldStatsText>, Without<TileInfoText>, Without<TurnInfoText>)>,
    world_info: Option<Res<WorldInfo>>,
    info_mode: Res<InfoDisplayMode>,
    civ_manager: Res<CivilizationManager>,
) {
    let Ok(window) = windows.single() else { return };
    let Ok((camera, camera_transform)) = camera_query.single() else { return };
    let Ok(mut info_text) = info_text_query.single_mut() else { return };
    let Ok(mut world_stats_text) = world_stats_query.single_mut() else { return };
    
    // Update world stats with civilization info
    if let Some(world_info) = world_info {
        let land_percent = (world_info.total_land_tiles as f32 / 
                          (world_info.total_land_tiles + world_info.total_ocean_tiles) as f32) * 100.0;
        
        let civ_count = civ_manager.civilizations.len();
        let total_cities: usize = civ_manager.civilizations.values().map(|c| c.cities.len()).sum();
        let total_units: usize = civ_manager.civilizations.values().map(|c| c.units.len()).sum();
        
        **world_stats_text = format!(
            "World Stats:\nSea Level: {:.3}\nLand: {:.1}% ({} tiles)\nOcean: {:.1}% ({} tiles)\nContinents: {}\nCivilizations: {}\nTotal Cities: {}\nTotal Units: {}",
            world_info.sea_level,
            land_percent, world_info.total_land_tiles,
            100.0 - land_percent, world_info.total_ocean_tiles,
            world_info.config.continent_count,
            civ_count,
            total_cities,
            total_units
        );
    }
    
    // Update tile info based on cursor position
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
    {
        let hovered_hex = HexCoord::from_world_pos(world_position, HEX_SIZE);
        
        if let Some(tile) = tile_query.iter().find(|t| t.hex_coord == hovered_hex) {
            let mut info = format_tile_info(tile, &info_mode);
            
            // Check for units on this tile
            let units_here: Vec<_> = unit_query.iter()
                .filter(|unit| unit.hex_coord == hovered_hex)
                .collect();
            
            if !units_here.is_empty() {
                info.push_str("\n\n=== UNITS ===");
                for unit in units_here {
                    let civ_name = civ_manager.get_civilization(unit.civilization_id)
                        .map(|c| c.name.as_str())
                        .unwrap_or("Unknown");
                    
                    info.push_str(&format!(
                        "\n{} ({}) - HP: {}/{}, MP: {}/{}",
                        unit.unit_type.get_name(),
                        civ_name,
                        unit.health,
                        unit.max_health,
                        unit.movement_points,
                        unit.max_movement_points
                    ));
                    
                    if unit.is_selected {
                        info.push_str(" [SELECTED]");
                    }
                }
            }
            
            // Check for cities on this tile
            let cities_here: Vec<_> = city_query.iter()
                .filter(|city| city.hex_coord == hovered_hex)
                .collect();
            
            if !cities_here.is_empty() {
                info.push_str("\n\n=== CITIES ===");
                for city in cities_here {
                    let civ_name = civ_manager.get_civilization(city.civilization_id)
                        .map(|c| c.name.as_str())
                        .unwrap_or("Unknown");
                    
                    info.push_str(&format!(
                        "\n{} ({}) - Pop: {}, Yields: F{:.1}/P{:.1}/S{:.1}/G{:.1}",
                        city.name,
                        civ_name,
                        city.population,
                        city.food_per_turn,
                        city.production_per_turn,
                        city.science_per_turn,
                        city.gold_per_turn
                    ));
                    
                    if city.is_capital {
                        info.push_str(" [CAPITAL]");
                    }
                }
            }
            
            **info_text = info;
        } else {
            **info_text = "".to_string();
        }
    } else {
        **info_text = "".to_string();
    }
}

fn format_tile_info(tile: &MapTile, mode: &InfoDisplayMode) -> String {
    let terrain_type = TerrainType::from_u8(tile.terrain);
    let biome_type = BiomeType::from_u8(tile.biome);
    
    let mut info = format!(
        "Coordinate: ({}, {})\nBiome: {:?}\nElevation: {:.2}m",
        tile.hex_coord.q, 
        tile.hex_coord.r, 
        biome_type,
        (tile.elevation_raw * 1000.0) // Convert to meters for display
    );
    
    match mode {
        InfoDisplayMode::Basic => {
            if tile.has_river {
                info.push_str(&format!("\nRiver Flow: {:.1}", tile.river_flow));
            }
            
            if tile.is_coastal {
                info.push_str("\nFeature: Coastal");
            }
            
            if tile.resource != 0 {
                let resource_type = ResourceType::from_u8(tile.resource);
                info.push_str(&format!("\nResource: {:?}", resource_type));
            }
        },
        
        InfoDisplayMode::Climate => {
            let climate_desc = get_climate_description(tile.temperature, tile.precipitation);
            info.push_str(&format!(
                "\nClimate: {}\nTemperature: {:.1}°\nPrecipitation: {:.0}mm\nSoil Fertility: {:.1}%",
                climate_desc,
                tile.temperature * 40.0 - 10.0, // Convert to rough Celsius
                tile.precipitation * 2000.0,     // Convert to mm per year
                tile.soil_fertility * 100.0
            ));
        },
        
        InfoDisplayMode::Resources => {
            let (food, production, science) = terrain_type.base_yields();
            let fertility_bonus = tile.soil_fertility * 2.0;
            let river_bonus = if tile.has_river { 1.0 } else { 0.0 };
            
            info.push_str(&format!(
                "\nBase Yields:\n  Food: {:.1} (+{:.1} fertility)\n  Production: {:.1}\n  Science: {:.1}\nRiver Bonus: +{:.1} food",
                food, fertility_bonus, production, science, river_bonus
            ));
            
            if tile.resource != 0 {
                let resource_type = ResourceType::from_u8(tile.resource);
                info.push_str(&format!("\nSpecial Resource: {:?}", resource_type));
            }
        },
        
        InfoDisplayMode::Suitability => {
            let suitability = evaluate_tile_suitability(tile);
            info.push_str(&format!(
                "\nSuitability Ratings:\n  Agriculture: {:.0}%\n  Industry: {:.0}%\n  Settlement: {:.0}%\n  Defense: {:.0}%",
                suitability.agriculture * 100.0,
                suitability.industry * 100.0,
                suitability.settlement * 100.0,
                suitability.defensibility * 100.0
            ));
            
            // Add geological info
            let geology_name = match tile.geology {
                0 => "Oceanic Crust",
                1 => "Continental Shelf", 
                2 => "Sedimentary",
                3 => "Igneous",
                4 => "Metamorphic",
                5 => "Volcanic",
                6 => "Limestone",
                7 => "Sandstone",
                8 => "Granite",
                9 => "Basalt",
                _ => "Unknown",
            };
            info.push_str(&format!("\nGeology: {}", geology_name));
        },
        
        InfoDisplayMode::Strategic => {
            // Strategic feature information
            if tile.strategic_feature != 0 {
                let feature = StrategicFeature::from_u8(tile.strategic_feature);
                info.push_str(&format!("\nStrategic Feature: {}", feature.name()));
            }
            
            info.push_str(&format!(
                "\nStrategic Values:\n  Defensibility: {:.0}%\n  Trade Value: {:.0}%\n  Naval Access: {:.0}%\n  Flood Risk: {:.0}%",
                tile.defensibility * 100.0,
                tile.trade_value * 100.0,
                tile.naval_access * 100.0,
                tile.flood_risk * 100.0
            ));
            
            // Additional strategic context
            if tile.has_river {
                info.push_str(&format!("\nRiver Flow: {:.1}", tile.river_flow));
            }
            
            if tile.flood_risk > 0.6 {
                info.push_str("\n⚠ High Flood Risk");
            }
            
            if tile.defensibility > 0.8 {
                info.push_str("\n🏰 Excellent Defense");
            }
            
            if tile.trade_value > 0.8 {
                info.push_str("\n💰 Prime Trade Location");
            }
        }
    }
    
    info
}

fn toggle_info_display(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut info_mode: ResMut<InfoDisplayMode>,
) {
    if keyboard.just_pressed(KeyCode::Tab) {
        *info_mode = match *info_mode {
            InfoDisplayMode::Basic => InfoDisplayMode::Climate,
            InfoDisplayMode::Climate => InfoDisplayMode::Resources,
            InfoDisplayMode::Resources => InfoDisplayMode::Suitability,
            InfoDisplayMode::Suitability => InfoDisplayMode::Strategic,
            InfoDisplayMode::Strategic => InfoDisplayMode::Basic,
        };
        
        let mode_name = match *info_mode {
            InfoDisplayMode::Basic => "Basic",
            InfoDisplayMode::Climate => "Climate",
            InfoDisplayMode::Resources => "Resources", 
            InfoDisplayMode::Suitability => "Suitability",
            InfoDisplayMode::Strategic => "Strategic",
        };
        
        println!("Info display mode: {}", mode_name);
    }
}

fn camera_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
    time: Res<Time>,
) {
    if let Ok(mut camera_transform) = camera_query.single_mut() {
        let mut movement_speed = 500.0;
        
        // Faster movement with shift
        if keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight) {
            movement_speed *= 2.0;
        }
        
        let mut direction = Vec3::ZERO;
        
        if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
            direction.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
            direction.y -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
            direction.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
            direction.x += 1.0;
        }
        
        if direction.length() > 0.0 {
            direction = direction.normalize();
            camera_transform.translation += direction * movement_speed * time.delta_secs();
        }
    }
}

fn basic_input(
    keyboard_input: Res<ButtonInput<KeyCode>>, 
    mut exit: EventWriter<AppExit>
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

fn hex_hover_system(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut tile_query: Query<(&MapTile, &mut MeshMaterial2d<ColorMaterial>), Without<Culled>>,
    terrain_assets: Res<TerrainAssets>,
    mut hover_state: ResMut<HoverState>,
) {
    let Ok(window) = windows.single() else { return };
    let Ok((camera, camera_transform)) = camera_query.single() else { return };
    
    // Determine what tile we're hovering over (if any)
    let new_hovered = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
        .map(|world_pos| HexCoord::from_world_pos(world_pos, HEX_SIZE));
    
    // Check if hover changed
    if hover_state.current_hovered != new_hovered {
        hover_state.previous_hovered = hover_state.current_hovered;
        hover_state.current_hovered = new_hovered;
        
        // Reset ALL tiles to their enhanced materials first (not base materials)
        for (tile, mut material_handle) in tile_query.iter_mut() {
            if let Some(enhanced_material) = terrain_assets.enhanced_materials.get(&tile.hex_coord) {
                material_handle.0 = enhanced_material.clone();
            }
        }
        
        // Now highlight ONLY the currently hovered tile (if any)
        if let Some(hovered_coord) = hover_state.current_hovered {
            for (tile, mut material_handle) in tile_query.iter_mut() {
                if tile.hex_coord == hovered_coord {
                    // Use the pre-computed hover material that preserves shading
                    if let Some(hover_material) = terrain_assets.hover_materials.get(&tile.hex_coord) {
                        material_handle.0 = hover_material.clone();
                    }
                    break; // Found the tile, no need to continue
                }
            }
        }
    }
}

fn debug_info_system(
    tile_query: Query<&MapTile, Without<Culled>>,
    culled_query: Query<&MapTile, With<Culled>>,
    unit_query: Query<&game::units::Unit>,
    city_query: Query<&game::cities::City>,
    keyboard: Res<ButtonInput<KeyCode>>,
    world_info: Option<Res<WorldInfo>>,
    civ_manager: Res<CivilizationManager>,
    game_state: Res<GameState>,
) {
    if keyboard.just_pressed(KeyCode::F3) {
        let visible_tiles = tile_query.iter().count();
        let culled_tiles = culled_query.iter().count();
        let total_tiles = visible_tiles + culled_tiles;
        
        let rivers = tile_query.iter().filter(|t| t.has_river).count();
        let coastal = tile_query.iter().filter(|t| t.is_coastal).count();
        let resources = tile_query.iter().filter(|t| t.resource != 0).count();
        
        println!("=== DEBUG INFO ===");
        println!("Game Turn: {}, Phase: {:?}", game_state.game_turn, game_state.current_phase);
        println!("Total tiles: {}", total_tiles);
        println!("Visible tiles: {}", visible_tiles);
        println!("Culled tiles: {}", culled_tiles);
        println!("Culling ratio: {:.1}%", (culled_tiles as f32 / total_tiles as f32) * 100.0);
        println!("Rivers: {}, Coastal: {}, Resources: {}", rivers, coastal, resources);
        
        // Civilization info
        println!("=== CIVILIZATIONS ===");
        for (id, civ) in &civ_manager.civilizations {
            println!("Civ {}: {} - Cities: {}, Units: {}, Gold: {:.0}, Science: {:.0}",
                     id, civ.name, civ.cities.len(), civ.units.len(), civ.gold, civ.science_points);
        }
        
        // Unit and city counts
        let total_units = unit_query.iter().count();
        let total_cities = city_query.iter().count();
        println!("Total Units: {}, Total Cities: {}", total_units, total_cities);
        
        // Calculate average climate values
        let total_temp: f32 = tile_query.iter().map(|t| t.temperature).sum();
        let total_precip: f32 = tile_query.iter().map(|t| t.precipitation).sum();
        let total_fertility: f32 = tile_query.iter().map(|t| t.soil_fertility).sum();
        
        let count = visible_tiles as f32;
        println!("=== CLIMATE AVERAGES ===");
        println!("Avg Temperature: {:.2}", total_temp / count);
        println!("Avg Precipitation: {:.2}", total_precip / count);
        println!("Avg Soil Fertility: {:.2}", total_fertility / count);
        
        // Show biome distribution for visible tiles
        if world_info.is_some() {
            println!("=== BIOME DISTRIBUTION (Visible) ===");
            let mut biome_counts = std::collections::HashMap::new();
            for tile in tile_query.iter() {
                *biome_counts.entry(tile.biome).or_insert(0) += 1;
            }
            
            let mut sorted_biomes: Vec<_> = biome_counts.iter().collect();
            sorted_biomes.sort_by(|a, b| b.1.cmp(a.1));
            
            for (biome_id, count) in sorted_biomes.iter().take(8) {
                let biome_type = BiomeType::from_u8(**biome_id);
                let percentage = (**count as f32 / visible_tiles as f32) * 100.0;
                println!("{:?}: {} ({:.1}%)", biome_type, count, percentage);
            }
        }
        
        // Visual config info
        println!("=== CONTROLS ===");
        println!("SPACE/ENTER: End turn | Click: Select/Move units | Tab: Info modes");
        println!("E: Toggle elevation shading | [ ]: Adjust intensity");
    }
}

// System wrapper functions for the terrain shading toggles
fn toggle_elevation_shading_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    terrain_assets: ResMut<TerrainAssets>,
    materials: ResMut<Assets<ColorMaterial>>,
    tile_query: Query<(Entity, &MapTile)>,
    tile_materials: Query<&mut MeshMaterial2d<ColorMaterial>>,
) {
    toggle_elevation_shading(keyboard, terrain_assets, materials, tile_query, tile_materials);
}

fn adjust_elevation_intensity_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    terrain_assets: ResMut<TerrainAssets>,
    materials: ResMut<Assets<ColorMaterial>>,
    tile_query: Query<(Entity, &MapTile)>,
    tile_materials: Query<&mut MeshMaterial2d<ColorMaterial>>,
) {
    adjust_elevation_intensity(keyboard, terrain_assets, materials, tile_query, tile_materials);
}