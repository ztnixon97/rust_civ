use bevy::prelude::*;
use super::hex::HexCoord;
use super::units::{Unit, UnitSelection};
use super::civilization::CivilizationManager;
use super::game_initialization::GameState;
use super::map::MapTile;
use super::world_gen::BiomeType;
use rand::Rng;

#[derive(Resource)]
pub struct CombatState {
    pub combat_preview: Option<CombatPreview>,
}

impl Default for CombatState {
    fn default() -> Self {
        Self {
            combat_preview: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CombatPreview {
    pub attacker_entity: Entity,
    pub defender_entity: Entity,
    pub attacker_strength: u32,
    pub defender_strength: u32,
    pub attacker_win_chance: f32,
    pub terrain_modifier: f32,
}

#[derive(Component)]
pub struct CombatResult {
    pub winner: Entity,
    pub loser: Entity,
    pub damage_dealt: u32,
    pub experience_gained: u32,
}

// System for handling combat initiation
pub fn combat_system(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut unit_query: Query<(Entity, &mut Unit)>,
    tile_query: Query<&MapTile>,
    unit_selection: Res<UnitSelection>,
    mut combat_state: ResMut<CombatState>,
    civ_manager: Res<CivilizationManager>,
    game_state: Res<GameState>,
) {
    if !game_state.is_initialized {
        return;
    }
    
    // Handle attack command with 'A' key
    if keyboard.just_pressed(KeyCode::KeyA) {
        if let Some(selected_unit_entity) = unit_selection.selected_unit {
            if let Ok((_, unit)) = unit_query.get(selected_unit_entity) {
                if unit.can_attack && unit.movement_points > 0 && !unit.has_attacked {
                    println!("Attack mode activated. Click on an enemy unit to attack.");
                    println!("Available targets within range will be highlighted.");
                    
                    // Show available attack targets
                    show_attack_targets(selected_unit_entity, &unit_query, &civ_manager);
                } else {
                    println!("Unit cannot attack (no movement, already attacked, or non-combat unit)!");
                }
            }
        } else {
            println!("No unit selected! Select a military unit to attack.");
        }
    }
    
    // Handle combat target selection with mouse click
    if mouse_input.just_pressed(MouseButton::Left) {
        handle_combat_targeting(
            &mut commands,
            &windows,
            &camera_query,
            &mut unit_query,
            &tile_query,
            &unit_selection,
            &mut combat_state,
            &civ_manager,
        );
    }
    
    // Handle combat confirmation with 'Enter'
    if keyboard.just_pressed(KeyCode::Enter) {
        if let Some(preview) = combat_state.combat_preview.take() {
            execute_combat(&mut commands, &mut unit_query, &tile_query, preview, &civ_manager);
        }
    }
    
    // Cancel combat preview with 'Escape'
    if keyboard.just_pressed(KeyCode::Escape) {
        if combat_state.combat_preview.is_some() {
            combat_state.combat_preview = None;
            println!("Combat cancelled.");
        }
    }
}

fn show_attack_targets(
    attacker_entity: Entity,
    unit_query: &Query<(Entity, &mut Unit)>,
    civ_manager: &CivilizationManager,
) {
    if let Ok((_, attacker)) = unit_query.get(attacker_entity) {
        let attack_range = get_attack_range(&attacker);
        let mut targets_found = 0;
        
        for (target_entity, target_unit) in unit_query.iter() {
            if target_entity == attacker_entity {
                continue;
            }
            
            // Check if target is enemy
            if are_enemies(attacker.civilization_id, target_unit.civilization_id, civ_manager) {
                let distance = hex_distance(attacker.hex_coord, target_unit.hex_coord);
                
                if distance <= attack_range {
                    targets_found += 1;
                    let target_civ_name = civ_manager.get_civilization(target_unit.civilization_id)
                        .map(|c| c.name.as_str())
                        .unwrap_or("Unknown");
                    
                    println!("  Target {}: {} {} at ({}, {}) - Distance: {}",
                             targets_found,
                             target_civ_name,
                             target_unit.unit_type.get_name(),
                             target_unit.hex_coord.q,
                             target_unit.hex_coord.r,
                             distance);
                }
            }
        }
        
        if targets_found == 0 {
            println!("No enemy units within attack range!");
        }
    }
}

fn handle_combat_targeting(
    commands: &mut Commands,
    windows: &Query<&Window>,
    camera_query: &Query<(&Camera, &GlobalTransform)>,
    unit_query: &mut Query<(Entity, &mut Unit)>,
    tile_query: &Query<&MapTile>,
    unit_selection: &Res<UnitSelection>,
    combat_state: &mut ResMut<CombatState>,
    civ_manager: &Res<CivilizationManager>,
) {
    let Ok(window) = windows.single() else { return };
    let Ok((camera, camera_transform)) = camera_query.single() else { return };
    
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
    {
        let clicked_hex = HexCoord::from_world_pos(world_position, super::map::HEX_SIZE);
        
        // Check if there's a unit at the clicked position
        if let Some(selected_entity) = unit_selection.selected_unit {
            if let Ok((attacker_entity, attacker)) = unit_query.get(selected_entity) {
                if !attacker.can_attack || attacker.movement_points == 0 || attacker.has_attacked {
                    return;
                }
                
                // Find target unit at clicked location
                for (target_entity, target_unit) in unit_query.iter() {
                    if target_unit.hex_coord == clicked_hex && target_entity != attacker_entity {
                        // Check if target is enemy
                        if are_enemies(attacker.civilization_id, target_unit.civilization_id, civ_manager) {
                            let distance = hex_distance(attacker.hex_coord, target_unit.hex_coord);
                            let attack_range = get_attack_range(&attacker);
                            
                            if distance <= attack_range {
                                // Create combat preview
                                let preview = create_combat_preview(
                                    attacker_entity,
                                    target_entity,
                                    &attacker,
                                    &target_unit,
                                    tile_query,
                                );
                                
                                display_combat_preview(&preview, civ_manager);
                                combat_state.combat_preview = Some(preview);
                                return;
                            } else {
                                println!("Target is out of range! (Distance: {}, Range: {})", distance, attack_range);
                            }
                        } else {
                            println!("Cannot attack allied units!");
                        }
                        break;
                    }
                }
            }
        }
    }
}

fn create_combat_preview(
    attacker_entity: Entity,
    defender_entity: Entity,
    attacker: &Unit,
    defender: &Unit,
    tile_query: &Query<&MapTile>,
) -> CombatPreview {
    let attacker_strength = attacker.get_combat_strength(true);
    let mut defender_strength = defender.get_combat_strength(false);
    
    // Apply terrain defensive bonuses
    let terrain_modifier = get_terrain_defensive_bonus(defender.hex_coord, tile_query);
    defender_strength = (defender_strength as f32 * terrain_modifier) as u32;
    
    // Calculate win probability (simplified)
    let total_strength = attacker_strength + defender_strength;
    let attacker_win_chance = if total_strength > 0 {
        attacker_strength as f32 / total_strength as f32
    } else {
        0.5
    };
    
    CombatPreview {
        attacker_entity,
        defender_entity,
        attacker_strength,
        defender_strength,
        attacker_win_chance,
        terrain_modifier,
    }
}

fn display_combat_preview(preview: &CombatPreview, _civ_manager: &CivilizationManager) {
    println!("=== COMBAT PREVIEW ===");
    println!("Attacker Strength: {}", preview.attacker_strength);
    println!("Defender Strength: {} (terrain bonus: {:.1}x)", 
             preview.defender_strength, preview.terrain_modifier);
    println!("Attacker Win Chance: {:.1}%", preview.attacker_win_chance * 100.0);
    println!("Press ENTER to attack, ESC to cancel");
}

fn execute_combat(
    commands: &mut Commands,
    unit_query: &mut Query<(Entity, &mut Unit)>,
    _tile_query: &Query<&MapTile>,
    preview: CombatPreview,
    _civ_manager: &CivilizationManager,
) {
    // We need to handle the borrowing more carefully
    let mut attacker_data = None;
    let mut defender_data = None;
    
    // First, get immutable references to calculate combat
    {
        let Ok((_, attacker)) = unit_query.get(preview.attacker_entity) else { return };
        let Ok((_, defender)) = unit_query.get(preview.defender_entity) else { return };
        
        let mut rng = rand::rng();
        let roll = rng.random::<f32>();
        
        let attacker_wins = roll < preview.attacker_win_chance;
        
        println!("=== COMBAT RESULT ===");
        println!("Roll: {:.3}, Win threshold: {:.3}", roll, preview.attacker_win_chance);
        
        if attacker_wins {
            let damage = calculate_damage(preview.attacker_strength, preview.defender_strength, true);
            defender_data = Some((damage, false)); // (damage, is_killed)
            attacker_data = Some((0, false)); // Attacker takes no damage when winning
            println!("Attacker wins! Defender takes {} damage.", damage);
        } else {
            let damage = calculate_damage(preview.defender_strength, preview.attacker_strength, false);
            attacker_data = Some((damage, false));
            defender_data = Some((0, false)); // Defender takes no damage when winning
            println!("Defender wins! Attacker takes {} damage.", damage);
        }
    }
    
    // Now apply the changes with mutable access
    if let Some((damage, _)) = attacker_data {
        if let Ok((_, mut attacker)) = unit_query.get_mut(preview.attacker_entity) {
            attacker.has_attacked = true;
            attacker.movement_points = attacker.movement_points.saturating_sub(1);
            attacker.take_damage(damage);
            attacker.gain_experience(1);
            
            if !attacker.is_dead() {
                attacker.gain_experience(if damage == 0 { 3 } else { 1 }); // Extra for winning
            }
        }
    }
    
    if let Some((damage, _)) = defender_data {
        if let Ok((_, mut defender)) = unit_query.get_mut(preview.defender_entity) {
            defender.take_damage(damage);
            defender.gain_experience(1);
            
            if !defender.is_dead() {
                defender.gain_experience(if damage == 0 { 2 } else { 1 }); // Extra for winning
            }
        }
    }
    
    // Handle unit destruction
    if let Ok((_, attacker)) = unit_query.get(preview.attacker_entity) {
        if attacker.is_dead() {
            println!("Attacker unit destroyed!");
            commands.entity(preview.attacker_entity).despawn();
        }
    }
    
    if let Ok((_, defender)) = unit_query.get(preview.defender_entity) {
        if defender.is_dead() {
            println!("Defender unit destroyed!");
            commands.entity(preview.defender_entity).despawn();
        }
    }
}

fn get_attack_range(unit: &Unit) -> i32 {
    match unit.unit_type {
        super::cities::UnitType::Archer => 2, // Archers can attack from range
        super::cities::UnitType::Trireme => 2, // Naval units have range
        _ => 1, // Most units are melee
    }
}

fn are_enemies(civ1: u32, civ2: u32, _civ_manager: &CivilizationManager) -> bool {
    // For now, all civilizations are enemies except with themselves
    // In a full game, you'd have a diplomacy system
    civ1 != civ2
}

fn get_terrain_defensive_bonus(coord: HexCoord, tile_query: &Query<&MapTile>) -> f32 {
    if let Some(tile) = tile_query.iter().find(|t| t.hex_coord == coord) {
        let biome = BiomeType::from_u8(tile.biome);
        
        let mut bonus = 1.0;
        
        // Terrain defensive bonuses
        match biome {
            BiomeType::TropicalRainforest | BiomeType::TemperateDeciduousForest => {
                bonus += 0.25; // 25% bonus in forests
            }
            BiomeType::AlpineTundra | BiomeType::MontaneForest => {
                bonus += 0.5; // 50% bonus in mountains
            }
            BiomeType::HotDesert | BiomeType::ColdDesert => {
                bonus += 0.1; // Small bonus in harsh terrain
            }
            _ => {}
        }
        
        // River defensive bonus
        if tile.has_river {
            bonus += 0.25;
        }
        
        // Elevation defensive bonus
        if tile.elevation_raw > 0.3 {
            bonus += 0.2;
        }
        
        bonus
    } else {
        1.0
    }
}

fn calculate_damage(winner_strength: u32, loser_strength: u32, attacker_won: bool) -> u32 {
    let strength_ratio = winner_strength as f32 / loser_strength.max(1) as f32;
    
    // Base damage ranges from 20-80% of max health
    let base_damage_percent = if attacker_won { 
        0.3 + (strength_ratio - 1.0) * 0.2 // Attacker damage
    } else {
        0.2 + (strength_ratio - 1.0) * 0.15 // Counter-attack damage
    };
    
    let damage_percent = base_damage_percent.clamp(0.15, 0.8);
    (100.0 * damage_percent) as u32
}

fn hex_distance(a: HexCoord, b: HexCoord) -> i32 {
    let dq = (a.q - b.q).abs();
    let dr = (a.r - b.r).abs();
    let ds = (-(a.q + a.r) + (b.q + b.r)).abs();
    (dq.max(dr)).max(ds)
}

// System to clean up dead units from civilization lists
pub fn cleanup_dead_units_system(
    mut commands: Commands,
    unit_query: Query<(Entity, &Unit)>,
    mut civ_manager: ResMut<CivilizationManager>,
) {
    let mut units_to_remove = Vec::new();
    
    for (entity, unit) in unit_query.iter() {
        if unit.is_dead() {
            units_to_remove.push((entity, unit.civilization_id));
        }
    }
    
    for (entity, civ_id) in units_to_remove {
        // Remove from civilization
        if let Some(civ) = civ_manager.get_civilization_mut(civ_id) {
            civ.remove_unit(entity);
        }
        
        // Despawn entity
        commands.entity(entity).despawn();
    }
}