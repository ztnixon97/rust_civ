use bevy::prelude::*;
use super::hex::HexCoord;
use super::map::MapTile;
use super::units::{Unit, UnitSelection};
use super::cities::City;
use super::civilization::CivilizationManager;
use super::game_initialization::GameState;

#[derive(Resource)]
pub struct CityFoundingState {
    pub founding_unit: Option<Entity>,
    pub potential_city_name: String,
}

impl Default for CityFoundingState {
    fn default() -> Self {
        Self {
            founding_unit: None,
            potential_city_name: String::new(),
        }
    }
}

// System for handling city founding
pub fn city_founding_system(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut unit_query: Query<(Entity, &mut Unit)>,
    city_query: Query<&City>,
    tile_query: Query<&MapTile>,
    unit_selection: Res<UnitSelection>,
    mut civ_manager: ResMut<CivilizationManager>,
    game_state: Res<GameState>,
    _founding_state: ResMut<CityFoundingState>,
) {
    if !game_state.is_initialized {
        return;
    }
    
    // Check for 'F' key to found city
    if keyboard.just_pressed(KeyCode::KeyF) {
        if let Some(selected_unit_entity) = unit_selection.selected_unit {
            if let Ok((unit_entity, unit)) = unit_query.get_mut(selected_unit_entity) {
                if unit.can_found_cities && unit.movement_points > 0 {
                    // Check if location is valid for city founding
                    if can_found_city_at(unit.hex_coord, &city_query, &tile_query) {
                        // Generate a city name
                        let city_name = generate_city_name(unit.civilization_id, &civ_manager, &city_query);
                        
                        // Create the city
                        let city = City::new(
                            city_name.clone(),
                            unit.hex_coord,
                            unit.civilization_id,
                            game_state.game_turn,
                            false, // Not a capital (first city is marked as capital during init)
                        );
                        
                        let city_entity = commands.spawn(city).id();
                        
                        // Add city to civilization
                        if let Some(civ) = civ_manager.get_civilization_mut(unit.civilization_id) {
                            civ.add_city(city_entity);
                        }
                        
                        // Remove the settler unit (they become the city)
                        if let Some(civ) = civ_manager.get_civilization_mut(unit.civilization_id) {
                            civ.remove_unit(unit_entity);
                        }
                        commands.entity(unit_entity).despawn();
                        
                        println!("Founded city {} at ({}, {})", city_name, unit.hex_coord.q, unit.hex_coord.r);
                    } else {
                        println!("Cannot found city here! Cities must be at least 3 tiles apart and on suitable land.");
                    }
                } else {
                    println!("Selected unit cannot found cities or has no movement points!");
                }
            }
        } else {
            println!("No unit selected! Select a settler to found a city.");
        }
    }
}

fn can_found_city_at(coord: HexCoord, city_query: &Query<&City>, tile_query: &Query<&MapTile>) -> bool {
    // Check if there's already a city here
    if city_query.iter().any(|city| city.hex_coord == coord) {
        return false;
    }
    
    // Check if the tile is suitable (must be land)
    if let Some(tile) = tile_query.iter().find(|t| t.hex_coord == coord) {
        let biome = super::world_gen::BiomeType::from_u8(tile.biome);
        if matches!(biome, super::world_gen::BiomeType::Ocean | super::world_gen::BiomeType::Lake) {
            return false;
        }
    } else {
        return false; // Tile doesn't exist
    }
    
    // Check minimum distance from other cities (at least 3 tiles)
    let min_distance = 3;
    for city in city_query.iter() {
        if hex_distance(coord, city.hex_coord) < min_distance {
            return false;
        }
    }
    
    true
}

fn generate_city_name(civ_id: u32, civ_manager: &CivilizationManager, city_query: &Query<&City>) -> String {
    // Get existing city names for this civilization
    let existing_names: std::collections::HashSet<String> = city_query.iter()
        .filter(|city| city.civilization_id == civ_id)
        .map(|city| city.name.clone())
        .collect();
    
    // Get civilization name for naming theme
    let civ_name = civ_manager.get_civilization(civ_id)
        .map(|c| c.name.as_str())
        .unwrap_or("Unknown");
    
    // City name lists by civilization theme
    let name_lists = get_city_names_for_civilization(civ_name);
    
    // Find an unused name
    for name in name_lists {
        if !existing_names.contains(name) {
            return name.to_string();
        }
    }
    
    // Fallback: generate numbered name
    let base_name = format!("{} City", civ_name);
    let mut counter = 1;
    loop {
        let name = if counter == 1 {
            base_name.clone()
        } else {
            format!("{} {}", base_name, counter)
        };
        
        if !existing_names.contains(&name) {
            return name;
        }
        counter += 1;
    }
}

fn get_city_names_for_civilization(civ_name: &str) -> Vec<&'static str> {
    match civ_name {
        "Roman Empire" => vec![
            "Rome", "Antium", "Cumae", "Neapolis", "Ravenna", "Mediolanum", 
            "Aquileia", "Augusta", "Tarentum", "Brundisium", "Capua", "Veii"
        ],
        "Egyptian Kingdom" => vec![
            "Thebes", "Memphis", "Alexandria", "Elephantine", "Syene", "Edfu",
            "Dendera", "Abydos", "Hermopolis", "Heracleopolis", "Bubastis", "Tanis"
        ],
        "Greek City-States" => vec![
            "Athens", "Sparta", "Corinth", "Thebes", "Argos", "Delphi",
            "Olympia", "Rhodes", "Syracuse", "Byzantium", "Ephesus", "Miletus"
        ],
        "Phoenician Traders" => vec![
            "Carthage", "Tyre", "Sidon", "Byblos", "Cadiz", "Utica",
            "Leptis Magna", "Hippo", "Hadrumetum", "Thapsus", "Rusadir", "Tingis"
        ],
        "Celtic Tribes" => vec![
            "Bibracte", "Alesia", "Gergovia", "Avaricum", "Noviodunum", "Lutetia",
            "Camulodunum", "Londinium", "Isca", "Mona", "Tara", "Emain Macha"
        ],
        "Mesopotamian Empire" => vec![
            "Babylon", "Ur", "Uruk", "Nippur", "Eridu", "Lagash",
            "Kish", "Akkad", "Mari", "Assur", "Nineveh", "Ctesiphon"
        ],
        _ => vec![
            "New Settlement", "Trading Post", "Riverside", "Hillfort", "Haven",
            "Crossroads", "Stronghold", "Meadowbrook", "Stonebridge", "Goldvale"
        ]
    }
}

// System for handling worker actions (building improvements)
pub fn worker_actions_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut unit_query: Query<&mut Unit>,
    unit_selection: Res<UnitSelection>,
    game_state: Res<GameState>,
) {
    if !game_state.is_initialized {
        return;
    }
    
    // Check for 'B' key to build improvement
    if keyboard.just_pressed(KeyCode::KeyB) {
        if let Some(selected_unit_entity) = unit_selection.selected_unit {
            if let Ok(mut unit) = unit_query.get_mut(selected_unit_entity) {
                if unit.can_build_improvements && unit.movement_points > 0 {
                    // For now, just consume movement points and show message
                    // In a full implementation, this would start an improvement construction
                    unit.movement_points = 0;
                    unit.has_moved = true;
                    
                    println!("Worker is building an improvement at ({}, {}). This will take several turns.", 
                             unit.hex_coord.q, unit.hex_coord.r);
                    println!("(Improvement system not fully implemented yet)");
                } else {
                    println!("Selected unit cannot build improvements or has no movement points!");
                }
            }
        } else {
            println!("No unit selected! Select a worker to build improvements.");
        }
    }
}

// System for skipping unit turns
pub fn skip_unit_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut unit_query: Query<&mut Unit>,
    unit_selection: Res<UnitSelection>,
    game_state: Res<GameState>,
) {
    if !game_state.is_initialized {
        return;
    }
    
    // Check for 'S' key to skip unit turn
    if keyboard.just_pressed(KeyCode::KeyS) {
        if let Some(selected_unit_entity) = unit_selection.selected_unit {
            if let Ok(mut unit) = unit_query.get_mut(selected_unit_entity) {
                unit.movement_points = 0;
                unit.has_moved = true;
                println!("Skipped turn for {} at ({}, {})", 
                         unit.unit_type.get_name(), unit.hex_coord.q, unit.hex_coord.r);
            }
        }
    }
}

// System for fortifying units
pub fn fortify_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut unit_query: Query<&mut Unit>,
    unit_selection: Res<UnitSelection>,
    game_state: Res<GameState>,
) {
    if !game_state.is_initialized {
        return;
    }
    
    // Check for 'Shift+F' to fortify (different from found city)
    if keyboard.just_pressed(KeyCode::KeyF) && 
       (keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight)) {
        if let Some(selected_unit_entity) = unit_selection.selected_unit {
            if let Ok(mut unit) = unit_query.get_mut(selected_unit_entity) {
                if unit.can_attack { // Only military units can fortify
                    unit.fortify();
                    println!("Unit fortified at ({}, {}). Defense bonus will increase each turn.", 
                             unit.hex_coord.q, unit.hex_coord.r);
                } else {
                    println!("Only military units can fortify!");
                }
            }
        }
    }
}

fn hex_distance(a: HexCoord, b: HexCoord) -> i32 {
    let dq = (a.q - b.q).abs();
    let dr = (a.r - b.r).abs();
    let ds = (-(a.q + a.r) + (b.q + b.r)).abs();
    (dq.max(dr)).max(ds)
}

// Helper function to check if a player has units that can still move
pub fn player_has_active_units(
    unit_query: &Query<&Unit>,
    civ_manager: &CivilizationManager,
) -> bool {
    if let Some(player_civ) = civ_manager.get_player_civilization() {
        unit_query.iter().any(|unit| {
            unit.civilization_id == player_civ.id && 
            unit.movement_points > 0 && 
            !unit.has_moved
        })
    } else {
        false
    }
}

// System to auto-advance turn when player has no more moves
pub fn auto_turn_advance_system(
    unit_query: Query<&Unit>,
    civ_manager: Res<CivilizationManager>,
    game_state: Res<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if !game_state.is_initialized {
        return;
    }
    
    // Only check during player turns
    if let super::game_initialization::GamePhase::PlayerTurn = game_state.current_phase {
        // Check if player pressed 'N' for next unit with moves
        if keyboard.just_pressed(KeyCode::KeyN) {
            if let Some(player_civ) = civ_manager.get_player_civilization() {
                let active_units: Vec<_> = unit_query.iter()
                    .filter(|unit| {
                        unit.civilization_id == player_civ.id && 
                        unit.movement_points > 0 && 
                        !unit.has_moved
                    })
                    .collect();
                
                if active_units.is_empty() {
                    println!("No units with remaining movement. Press SPACE to end turn.");
                } else {
                    println!("You have {} units that can still move:", active_units.len());
                    for (i, unit) in active_units.iter().enumerate() {
                        println!("  {}. {} at ({}, {}) - {} movement points", 
                                 i + 1,
                                 unit.unit_type.get_name(),
                                 unit.hex_coord.q, 
                                 unit.hex_coord.r,
                                 unit.movement_points);
                    }
                }
            }
        }
    }
}