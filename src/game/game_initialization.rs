use bevy::prelude::*;
use super::hex::HexCoord;
use super::map::MapTile;
use super::world_gen::BiomeType;
use super::civilization::{CivilizationManager, create_default_civilizations};
use super::cities::{City, UnitType};
use super::units::{Unit, spawn_unit, spawn_city};

#[derive(Resource)]
pub struct GameState {
    pub is_initialized: bool,
    pub game_turn: u32,
    pub current_phase: GamePhase,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GamePhase {
    Initialization,
    PlayerTurn,
    AITurn(u32), // Civilization ID
    EndTurn,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            is_initialized: false,
            game_turn: 1,
            current_phase: GamePhase::Initialization,
        }
    }
}

// System to initialize the game once the world is generated
pub fn initialize_game(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    mut civ_manager: ResMut<CivilizationManager>,
    tile_query: Query<&MapTile>,
    world_info: Option<Res<super::map::WorldInfo>>,
) {
    if game_state.is_initialized {
        return;
    }
    
    // Wait for world generation to complete
    if world_info.is_none() || tile_query.is_empty() {
        return;
    }
    
    println!("=== INITIALIZING CIVILIZATION GAME ===");
    
    // Create civilizations
    let civilizations = create_default_civilizations();
    let mut civ_ids = Vec::new();
    
    for civ in civilizations {
        let id = civ_manager.add_civilization(civ);
        civ_ids.push(id);
    }
    
    // Find suitable starting positions for each civilization
    let starting_positions = find_starting_positions(&tile_query, civ_ids.len());
    
    if starting_positions.len() < civ_ids.len() {
        println!("Warning: Could only find {} starting positions for {} civilizations", 
                 starting_positions.len(), civ_ids.len());
    }
    
    // Spawn starting cities and units for each civilization
    for (i, &civ_id) in civ_ids.iter().enumerate() {
        if let Some(&start_pos) = starting_positions.get(i) {
            spawn_civilization_start(&mut commands, civ_id, start_pos, &mut civ_manager);
        }
    }
    
    // Set up turn order
    if !civ_ids.is_empty() {
        civ_manager.current_turn_civ = civ_ids[0];
    }
    
    game_state.is_initialized = true;
    game_state.current_phase = GamePhase::PlayerTurn;
    
    println!("Game initialized with {} civilizations", civ_ids.len());
    print_game_status(&civ_manager);
}

fn find_starting_positions(tile_query: &Query<&MapTile>, num_civs: usize) -> Vec<HexCoord> {
    let mut candidates = Vec::new();
    let mut positions = Vec::new();
    
    // First pass: find all suitable starting tiles
    for tile in tile_query.iter() {
        if is_good_starting_position(tile, tile_query) {
            candidates.push((tile.hex_coord, rate_starting_position(tile, tile_query)));
        }
    }
    
    // Sort by quality (best first)
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    // Select positions ensuring minimum distance between civilizations
    let min_distance = 15; // Minimum hex distance between starting positions
    
    for (coord, _score) in &candidates {
        let too_close = positions.iter().any(|&existing| {
            hex_distance(*coord, existing) < min_distance
        });
        
        if !too_close {
            positions.push(*coord);
            if positions.len() >= num_civs {
                break;
            }
        }
    }
    
    // If we couldn't find enough well-spaced positions, relax the distance requirement
    if positions.len() < num_civs {
        let relaxed_distance = 10;
        for (coord, _score) in candidates {
            if positions.contains(&coord) {
                continue;
            }
            
            let too_close = positions.iter().any(|&existing| {
                hex_distance(coord, existing) < relaxed_distance
            });
            
            if !too_close {
                positions.push(coord);
                if positions.len() >= num_civs {
                    break;
                }
            }
        }
    }
    
    positions
}

fn is_good_starting_position(tile: &MapTile, tile_query: &Query<&MapTile>) -> bool {
    // Must be on land
    let biome = BiomeType::from_u8(tile.biome);
    if matches!(biome, BiomeType::Ocean | BiomeType::Lake) {
        return false;
    }
    
    // Avoid extreme biomes for starting positions
    match biome {
        BiomeType::HotDesert | BiomeType::ColdDesert => return false,
        BiomeType::TundraBarren | BiomeType::AlpineTundra => return false,
        BiomeType::TropicalRainforest => return false, // Too dense for starting
        _ => {}
    }
    
    // Check for basic necessities in the immediate area
    let has_freshwater = tile.has_river || has_freshwater_nearby(tile.hex_coord, tile_query);
    let has_decent_fertility = tile.soil_fertility > 0.3;
    let not_too_harsh = tile.temperature > 0.2 && tile.precipitation > 0.15;
    
    has_freshwater && has_decent_fertility && not_too_harsh
}

fn rate_starting_position(tile: &MapTile, tile_query: &Query<&MapTile>) -> f32 {
    let mut score = 0.0;
    
    // Base fertility score
    score += tile.soil_fertility * 30.0;
    
    // Climate preference (temperate is best)
    let climate_score = match (tile.temperature, tile.precipitation) {
        (t, p) if t > 0.4 && t < 0.7 && p > 0.3 && p < 0.8 => 20.0, // Ideal temperate
        (t, p) if t > 0.3 && t < 0.8 && p > 0.2 && p < 0.9 => 15.0, // Good
        _ => 5.0, // Acceptable
    };
    score += climate_score;
    
    // Freshwater bonus
    if tile.has_river {
        score += 15.0;
    } else if has_freshwater_nearby(tile.hex_coord, tile_query) {
        score += 10.0;
    }
    
    // Coastal bonus (for trade and naval expansion)
    if tile.is_coastal {
        score += 10.0;
    }
    
    // Resource bonus
    if tile.resource != 0 {
        score += 8.0;
    }
    
    // Strategic position bonus
    score += tile.defensibility * 5.0;
    score += tile.trade_value * 5.0;
    
    // Nearby tile diversity and quality
    let nearby_score = rate_nearby_tiles(tile.hex_coord, tile_query);
    score += nearby_score;
    
    // Biome preference
    let biome = BiomeType::from_u8(tile.biome);
    let biome_bonus = match biome {
        BiomeType::TemperateGrassland => 15.0,           // Excellent for agriculture
        BiomeType::TemperateDeciduousForest => 12.0,     // Good balance
        BiomeType::TropicalGrasslandSavanna => 10.0,     // Good for expansion
        BiomeType::TemperateRainforest => 8.0,           // High productivity
        BiomeType::TropicalSeasonalForest => 8.0,        // Decent
        BiomeType::Shrubland => 5.0,                     // Acceptable
        _ => 0.0,
    };
    score += biome_bonus;
    
    score
}

fn has_freshwater_nearby(center: HexCoord, tile_query: &Query<&MapTile>) -> bool {
    for neighbor in center.neighbors() {
        if let Some(tile) = tile_query.iter().find(|t| t.hex_coord == neighbor) {
            if tile.has_river || matches!(BiomeType::from_u8(tile.biome), BiomeType::Lake) {
                return true;
            }
        }
    }
    false
}

fn rate_nearby_tiles(center: HexCoord, tile_query: &Query<&MapTile>) -> f32 {
    let mut score = 0.0;
    let mut _tile_count = 0;
    
    // Check tiles within 2 hex radius
    for tile in tile_query.iter() {
        let distance = hex_distance(center, tile.hex_coord);
        if distance <= 2 && distance > 0 {
            _tile_count += 1;
            
            // Distance weight (closer tiles matter more)
            let weight = match distance {
                1 => 1.0,
                2 => 0.5,
                _ => 0.0,
            };
            
            // Land tile bonus
            let biome = BiomeType::from_u8(tile.biome);
            if !matches!(biome, BiomeType::Ocean) {
                score += 2.0 * weight;
            }
            
            // Fertility
            score += tile.soil_fertility * 3.0 * weight;
            
            // Resources
            if tile.resource != 0 {
                score += 2.0 * weight;
            }
            
            // Terrain diversity bonus
            match biome {
                BiomeType::TemperateDeciduousForest | BiomeType::TaigaBorealForest => {
                    score += 1.0 * weight; // Production potential
                }
                BiomeType::TemperateGrassland | BiomeType::TropicalGrasslandSavanna => {
                    score += 1.5 * weight; // Food potential
                }
                BiomeType::AlpineTundra | BiomeType::MontaneForest => {
                    score += 0.5 * weight; // Mining potential
                }
                _ => {}
            }
        }
    }
    
    score
}

fn spawn_civilization_start(
    commands: &mut Commands,
    civ_id: u32,
    start_pos: HexCoord,
    civ_manager: &mut CivilizationManager,
) {
    let civ_name = civ_manager.get_civilization(civ_id)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    
    println!("Spawning {} at ({}, {})", civ_name, start_pos.q, start_pos.r);
    
    // Spawn capital city
    let capital_name = format!("{} Capital", civ_name);
    let _city_entity = spawn_city(
        commands,
        capital_name,
        start_pos,
        civ_id,
        1, // Founded on turn 1
        true, // Is capital
        civ_manager,
    );
    
    // Spawn starting units around the capital
    let starting_units = get_starting_units_for_civilization(civ_id, civ_manager);
    
    let mut spawn_positions = vec![start_pos];
    
    // Add neighboring positions for additional units
    let neighbors = start_pos.neighbors();
    spawn_positions.extend_from_slice(&neighbors);
    
    for (i, unit_type) in starting_units.iter().enumerate() {
        if let Some(&spawn_pos) = spawn_positions.get(i) {
            spawn_unit(commands, *unit_type, civ_id, spawn_pos, civ_manager);
        }
    }
}

fn get_starting_units_for_civilization(
    civ_id: u32,
    civ_manager: &CivilizationManager,
) -> Vec<UnitType> {
    let mut units = vec![
        UnitType::Settler,  // For founding additional cities
        UnitType::Warrior,  // Basic military unit
        UnitType::Scout,    // For exploration
        UnitType::Worker,   // For improvements
    ];
    
    // Add civilization-specific starting units
    if let Some(civ) = civ_manager.get_civilization(civ_id) {
        match civ.civ_type {
            super::civilization::CivilizationType::Military => {
                units.push(UnitType::Warrior); // Extra warrior
            }
            super::civilization::CivilizationType::Maritime => {
                units.push(UnitType::Galley); // Starting naval unit
            }
            super::civilization::CivilizationType::Commercial => {
                units.push(UnitType::Worker); // Extra worker for development
            }
            super::civilization::CivilizationType::Agricultural => {
                units.push(UnitType::Settler); // Extra settler for expansion
            }
            _ => {
                units.push(UnitType::Archer); // Default extra unit
            }
        }
    }
    
    units
}

fn hex_distance(a: HexCoord, b: HexCoord) -> i32 {
    let dq = (a.q - b.q).abs();
    let dr = (a.r - b.r).abs();
    let ds = (-(a.q + a.r) + (b.q + b.r)).abs();
    (dq.max(dr)).max(ds)
}

fn print_game_status(civ_manager: &CivilizationManager) {
    println!("=== GAME STATUS ===");
    println!("Turn: {}", civ_manager.turn_number);
    println!("Current Turn: Civilization {}", civ_manager.current_turn_civ);
    
    for (id, civ) in &civ_manager.civilizations {
        println!("Civilization {}: {} led by {} ({} cities, {} units)",
                 id,
                 civ.name,
                 civ.leader_name,
                 civ.cities.len(),
                 civ.units.len());
    }
}

// System for advancing turns
pub fn turn_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
    mut civ_manager: ResMut<CivilizationManager>,
    mut city_query: Query<&mut City>,
    mut unit_query: Query<&mut Unit>,
) {
    if !game_state.is_initialized {
        return;
    }
    
    // Handle turn advancement
    if keyboard.just_pressed(KeyCode::Space) || 
       keyboard.just_pressed(KeyCode::Enter) {
        advance_turn(&mut game_state, &mut civ_manager, &mut city_query, &mut unit_query);
    }
}

fn advance_turn(
    game_state: &mut ResMut<GameState>,
    civ_manager: &mut ResMut<CivilizationManager>,
    city_query: &mut Query<&mut City>,
    unit_query: &mut Query<&mut Unit>,
) {
    println!("Advancing turn...");
    
    // Process current civilization's end-of-turn activities
    let current_civ_id = civ_manager.current_turn_civ;
    
    // Process cities for the current civilization
    for mut city in city_query.iter_mut() {
        if city.civilization_id == current_civ_id {
            city.process_turn(civ_manager);
        }
    }
    
    // Process units for the current civilization
    for mut unit in unit_query.iter_mut() {
        if unit.civilization_id == current_civ_id {
            unit.start_turn();
        }
    }
    
    // Advance to next civilization
    civ_manager.next_turn();
    
    // Update game state
    if civ_manager.current_turn_civ == 1 { // Back to first civ = new turn
        game_state.game_turn = civ_manager.turn_number;
    }
    
    // Determine current phase
    if let Some(current_civ) = civ_manager.get_civilization(civ_manager.current_turn_civ) {
        game_state.current_phase = if current_civ.is_player {
            GamePhase::PlayerTurn
        } else {
            GamePhase::AITurn(current_civ.id)
        };
    }
    
    println!("Now: Turn {}, Civilization {} ({})",
             game_state.game_turn,
             civ_manager.current_turn_civ,
             civ_manager.get_civilization(civ_manager.current_turn_civ)
                 .map(|c| c.name.as_str())
                 .unwrap_or("Unknown"));
}

// System for AI turns (simplified - just advances automatically for now)
pub fn ai_turn_system(
    mut game_state: ResMut<GameState>,
    mut civ_manager: ResMut<CivilizationManager>,
    mut city_query: Query<&mut City>,
    mut unit_query: Query<&mut Unit>,
    time: Res<Time>,
) {
    if !game_state.is_initialized {
        return;
    }
    
    // For now, AI turns advance automatically after a short delay
    if let GamePhase::AITurn(_civ_id) = game_state.current_phase {
        // In a real implementation, this would be replaced with AI logic
        // For now, just auto-advance after 1 second
        static mut AI_TIMER: f32 = 0.0;
        unsafe {
            AI_TIMER += time.delta_secs();
            if AI_TIMER >= 1.0 {
                AI_TIMER = 0.0;
                advance_turn(&mut game_state, &mut civ_manager, &mut city_query, &mut unit_query);
            }
        }
    }
}

// Helper system to display current turn info
pub fn display_turn_info(
    game_state: Res<GameState>,
    civ_manager: Res<CivilizationManager>,
    mut turn_info_query: Query<&mut Text, With<TurnInfoText>>,
) {
    if !game_state.is_initialized {
        return;
    }
    
    if let Ok(mut text) = turn_info_query.single_mut() {
        let current_civ_name = civ_manager.get_civilization(civ_manager.current_turn_civ)
            .map(|c| c.name.as_str())
            .unwrap_or("Unknown");
        
        let phase_text = match game_state.current_phase {
            GamePhase::PlayerTurn => "Your Turn",
            GamePhase::AITurn(_) => "AI Turn",
            _ => "Processing",
        };
        
        **text = format!(
            "Turn {}: {} ({})\nPress SPACE/ENTER to end turn",
            game_state.game_turn,
            current_civ_name,
            phase_text
        );
    }
}

#[derive(Component)]
pub struct TurnInfoText;

// Function to add turn info UI to the setup
pub fn setup_turn_info_ui(mut commands: Commands) {
    // Turn info display (top center)
    commands.spawn((
        TurnInfoText,
        Text::new("Game Initializing..."),
        TextLayout::new_with_justify(JustifyText::Center),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-150.0)), // Center the 300px wide text
            width: Val::Px(300.0),
            ..default()
        },
    ));
}