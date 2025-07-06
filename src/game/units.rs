use bevy::prelude::*;
use super::hex::HexCoord;
use super::map::{MapTile, TerrainType};
use super::civilization::{CivilizationManager, CivTrait};
use super::cities::{UnitType, City};

#[derive(Component)]
pub struct Unit {
    pub unit_type: UnitType,
    pub civilization_id: u32,
    pub hex_coord: HexCoord,
    pub name: String,
    
    // Combat stats
    pub health: u32,
    pub max_health: u32,
    pub attack_strength: u32,
    pub defense_strength: u32,
    pub combat_experience: u32,
    
    // Movement
    pub movement_points: u32,
    pub max_movement_points: u32,
    pub movement_type: MovementType,
    
    // Special abilities
    pub can_found_cities: bool,
    pub can_build_improvements: bool,
    pub can_attack: bool,
    pub naval_unit: bool,
    
    // State
    pub has_moved: bool,
    pub has_attacked: bool,
    pub is_fortified: bool,
    pub fortification_turns: u32,
    pub is_selected: bool,
    
    // Production info (for display)
    pub turns_to_build: u32,
    pub production_cost: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MovementType {
    Land,           // Normal land movement
    Naval,          // Water-only movement
    Amphibious,     // Can move on land and water
    Air,            // Can fly over terrain (future use)
}

#[derive(Component)]
pub struct UnitMarker {
    pub civilization_id: u32,
    pub unit_type: UnitType,
}

#[derive(Component)]
pub struct Selected;

#[derive(Component)]
pub struct MovementIndicator;

#[derive(Resource)]
pub struct UnitSelection {
    pub selected_unit: Option<Entity>,
    pub valid_moves: Vec<HexCoord>,
    pub movement_indicators: Vec<Entity>,
}

impl Default for UnitSelection {
    fn default() -> Self {
        Self {
            selected_unit: None,
            valid_moves: Vec::new(),
            movement_indicators: Vec::new(),
        }
    }
}

impl Unit {
    pub fn new(unit_type: UnitType, civilization_id: u32, hex_coord: HexCoord) -> Self {
        let stats = unit_type.get_stats();
        
        Self {
            unit_type,
            civilization_id,
            hex_coord,
            name: format!("{:?}", unit_type), // Can be customized later
            health: stats.max_health,
            max_health: stats.max_health,
            attack_strength: stats.attack,
            defense_strength: stats.defense,
            combat_experience: 0,
            movement_points: stats.movement,
            max_movement_points: stats.movement,
            movement_type: stats.movement_type,
            can_found_cities: stats.can_found_cities,
            can_build_improvements: stats.can_build_improvements,
            can_attack: stats.can_attack,
            naval_unit: stats.naval_unit,
            has_moved: false,
            has_attacked: false,
            is_fortified: false,
            fortification_turns: 0,
            is_selected: false,
            turns_to_build: stats.build_time,
            production_cost: stats.production_cost,
        }
    }
    
    pub fn can_move_to(&self, target: HexCoord, tile_query: &Query<&MapTile>) -> bool {
        // Check if unit can enter this tile type
        if let Some(tile) = tile_query.iter().find(|t| t.hex_coord == target) {
            let terrain = TerrainType::from_u8(tile.terrain);
            
            match self.movement_type {
                MovementType::Land => !matches!(terrain, 
                    TerrainType::Ocean | TerrainType::Lake | TerrainType::River),
                MovementType::Naval => matches!(terrain, 
                    TerrainType::Ocean | TerrainType::Lake | TerrainType::River),
                MovementType::Amphibious => true, // Can go anywhere
                MovementType::Air => true,        // Can fly over anything
            }
        } else {
            false // Off-map
        }
    }
    
    pub fn get_movement_cost(&self, target: HexCoord, tile_query: &Query<&MapTile>) -> u32 {
        if let Some(tile) = tile_query.iter().find(|t| t.hex_coord == target) {
            let terrain = TerrainType::from_u8(tile.terrain);
            
            // Base movement cost by terrain
            let base_cost = match terrain {
                TerrainType::TemperateGrassland | TerrainType::TropicalGrasslandSavanna => 1,
                TerrainType::TemperateDeciduousForest | TerrainType::TropicalSeasonalForest => 2,
                TerrainType::TropicalRainforest | TerrainType::TaigaBorealForest => 2,
                TerrainType::AlpineTundra | TerrainType::MontaneForest => 3,
                TerrainType::HotDesert | TerrainType::ColdDesert => 2,
                TerrainType::TundraBarren | TerrainType::TundraWet => 2,
                TerrainType::Shrubland => 1,
                TerrainType::Wetland | TerrainType::Mangrove => 2,
                TerrainType::Ocean | TerrainType::Lake | TerrainType::River => 1, // For naval units
                _ => 1,
            };
            
            // River crossing penalty for land units
            let river_penalty = if tile.has_river && 
                                   self.movement_type == MovementType::Land && 
                                   !matches!(terrain, TerrainType::River) {
                1 // Extra movement point to cross river
            } else {
                0
            };
            
            base_cost + river_penalty
        } else {
            99 // Can't move off-map
        }
    }
    
    pub fn calculate_valid_moves(&self, tile_query: &Query<&MapTile>) -> Vec<HexCoord> {
        let mut valid_moves = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        
        // Start from current position
        queue.push_back((self.hex_coord, self.movement_points));
        visited.insert(self.hex_coord);
        
        while let Some((current_coord, remaining_movement)) = queue.pop_front() {
            for neighbor in current_coord.neighbors() {
                if visited.contains(&neighbor) {
                    continue;
                }
                
                if self.can_move_to(neighbor, tile_query) {
                    let movement_cost = self.get_movement_cost(neighbor, tile_query);
                    
                    if movement_cost <= remaining_movement {
                        valid_moves.push(neighbor);
                        visited.insert(neighbor);
                        
                        // Continue exploring from this position
                        let new_remaining = remaining_movement - movement_cost;
                        if new_remaining > 0 {
                            queue.push_back((neighbor, new_remaining));
                        }
                    }
                }
            }
        }
        
        valid_moves
    }
    
    pub fn move_to(&mut self, target: HexCoord, tile_query: &Query<&MapTile>) -> bool {
        if self.can_move_to(target, tile_query) {
            let movement_cost = self.get_movement_cost(target, tile_query);
            
            if movement_cost <= self.movement_points {
                self.hex_coord = target;
                self.movement_points -= movement_cost;
                self.has_moved = true;
                
                // Remove fortification when moving
                self.is_fortified = false;
                self.fortification_turns = 0;
                
                return true;
            }
        }
        false
    }
    
    pub fn fortify(&mut self) {
        self.is_fortified = true;
        self.fortification_turns = 0;
        self.movement_points = 0; // Spend all movement to fortify
    }
    
    pub fn get_combat_strength(&self, is_attacking: bool) -> u32 {
        let base_strength = if is_attacking {
            self.attack_strength
        } else {
            self.defense_strength
        };
        
        let mut total_strength = base_strength;
        
        // Experience bonus (5% per level)
        let experience_bonus = (self.combat_experience / 10) * 5;
        total_strength = (total_strength * (100 + experience_bonus)) / 100;
        
        // Fortification bonus for defenders
        if !is_attacking && self.is_fortified {
            let fortification_bonus = (self.fortification_turns.min(3) * 5).min(25); // Max 25% bonus
            total_strength = (total_strength * (100 + fortification_bonus)) / 100;
        }
        
        // Health penalty
        if self.health < self.max_health {
            let health_ratio = self.health as f32 / self.max_health as f32;
            total_strength = (total_strength as f32 * health_ratio) as u32;
        }
        
        total_strength.max(1) // Minimum 1 strength
    }
    
    pub fn take_damage(&mut self, damage: u32) {
        self.health = self.health.saturating_sub(damage);
    }
    
    pub fn gain_experience(&mut self, amount: u32) {
        self.combat_experience += amount;
        
        // Check for promotion (every 10 experience points)
        if self.combat_experience >= 10 && (self.combat_experience - amount) < 10 {
            self.promote();
        }
    }
    
    fn promote(&mut self) {
        // Simple promotion - increase both attack and defense
        self.attack_strength += 1;
        self.defense_strength += 1;
        println!("Unit {} has been promoted! New stats: ATK {}, DEF {}", 
                 self.name, self.attack_strength, self.defense_strength);
    }
    
    pub fn start_turn(&mut self) {
        self.movement_points = self.max_movement_points;
        self.has_moved = false;
        self.has_attacked = false;
        
        if self.is_fortified {
            self.fortification_turns += 1;
        }
        
        // Heal if not in combat (simplified)
        if self.health < self.max_health && self.is_fortified {
            self.health = (self.health + 1).min(self.max_health);
        }
    }
    
    pub fn is_dead(&self) -> bool {
        self.health == 0
    }
}

#[derive(Debug)]
pub struct UnitStats {
    pub max_health: u32,
    pub attack: u32,
    pub defense: u32,
    pub movement: u32,
    pub movement_type: MovementType,
    pub can_found_cities: bool,
    pub can_build_improvements: bool,
    pub can_attack: bool,
    pub naval_unit: bool,
    pub build_time: u32,
    pub production_cost: u32,
}

impl UnitType {
    pub fn get_stats(&self) -> UnitStats {
        match self {
            UnitType::Warrior => UnitStats {
                max_health: 100,
                attack: 2,
                defense: 1,
                movement: 1,
                movement_type: MovementType::Land,
                can_found_cities: false,
                can_build_improvements: false,
                can_attack: true,
                naval_unit: false,
                build_time: 3,
                production_cost: 15,
            },
            UnitType::Archer => UnitStats {
                max_health: 60,
                attack: 3,
                defense: 2,
                movement: 1,
                movement_type: MovementType::Land,
                can_found_cities: false,
                can_build_improvements: false,
                can_attack: true,
                naval_unit: false,
                build_time: 4,
                production_cost: 25,
            },
            UnitType::Spearman => UnitStats {
                max_health: 100,
                attack: 1,
                defense: 3,
                movement: 1,
                movement_type: MovementType::Land,
                can_found_cities: false,
                can_build_improvements: false,
                can_attack: true,
                naval_unit: false,
                build_time: 5,
                production_cost: 35,
            },
            UnitType::Settler => UnitStats {
                max_health: 100,
                attack: 0,
                defense: 1,
                movement: 2,
                movement_type: MovementType::Land,
                can_found_cities: true,
                can_build_improvements: false,
                can_attack: false,
                naval_unit: false,
                build_time: 10,
                production_cost: 100,
            },
            UnitType::Worker => UnitStats {
                max_health: 100,
                attack: 0,
                defense: 1,
                movement: 2,
                movement_type: MovementType::Land,
                can_found_cities: false,
                can_build_improvements: true,
                can_attack: false,
                naval_unit: false,
                build_time: 6,
                production_cost: 60,
            },
            UnitType::Scout => UnitStats {
                max_health: 100,
                attack: 1,
                defense: 1,
                movement: 2,
                movement_type: MovementType::Land,
                can_found_cities: false,
                can_build_improvements: false,
                can_attack: true,
                naval_unit: false,
                build_time: 3,
                production_cost: 15,
            },
            UnitType::Galley => UnitStats {
                max_health: 100,
                attack: 1,
                defense: 1,
                movement: 3,
                movement_type: MovementType::Naval,
                can_found_cities: false,
                can_build_improvements: false,
                can_attack: true,
                naval_unit: true,
                build_time: 5,
                production_cost: 40,
            },
            UnitType::Trireme => UnitStats {
                max_health: 100,
                attack: 2,
                defense: 1,
                movement: 4,
                movement_type: MovementType::Naval,
                can_found_cities: false,
                can_build_improvements: false,
                can_attack: true,
                naval_unit: true,
                build_time: 7,
                production_cost: 60,
            },
        }
    }
    
    pub fn get_symbol(&self) -> &'static str {
        match self {
            UnitType::Warrior => "⚔",
            UnitType::Archer => "🏹",
            UnitType::Spearman => "🗡",
            UnitType::Settler => "🏠",
            UnitType::Worker => "🔨",
            UnitType::Scout => "👁",
            UnitType::Galley => "⛵",
            UnitType::Trireme => "🚢",
        }
    }
    
    pub fn get_name(&self) -> &'static str {
        match self {
            UnitType::Warrior => "Warrior",
            UnitType::Archer => "Archer",
            UnitType::Spearman => "Spearman",
            UnitType::Settler => "Settler",
            UnitType::Worker => "Worker",
            UnitType::Scout => "Scout",
            UnitType::Galley => "Galley",
            UnitType::Trireme => "Trireme",
        }
    }
}

// System for handling unit selection
pub fn unit_selection_system(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut unit_query: Query<(Entity, &mut Unit), With<Unit>>,
    mut unit_selection: ResMut<UnitSelection>,
    tile_query: Query<&MapTile>,
    mut commands: Commands,
    civ_manager: Res<CivilizationManager>,
) {
    if !mouse_input.just_pressed(MouseButton::Left) {
        return;
    }
    
    let Ok(window) = windows.single() else { return };
    let Ok((camera, camera_transform)) = camera_query.single() else { return };
    
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
    {
        let clicked_hex = HexCoord::from_world_pos(world_position, super::map::HEX_SIZE);
        
        // Check if we clicked on a unit
        let mut clicked_unit = None;
        for (entity, unit) in unit_query.iter() {
            if unit.hex_coord == clicked_hex {
                // Only select units belonging to the current player's civilization
                if let Some(player_civ) = civ_manager.get_player_civilization() {
                    if unit.civilization_id == player_civ.id {
                        clicked_unit = Some(entity);
                        break;
                    }
                }
            }
        }
        
        if let Some(unit_entity) = clicked_unit {
            // Select the unit
            select_unit(unit_entity, &mut unit_selection, &mut unit_query, &tile_query, &mut commands);
        } else if let Some(selected_entity) = unit_selection.selected_unit {
            // Try to move the selected unit
            if let Ok((_, mut unit)) = unit_query.get_mut(selected_entity) {
                if unit_selection.valid_moves.contains(&clicked_hex) {
                    unit.move_to(clicked_hex, &tile_query);
                    // Update the unit's visual position would happen in another system
                }
            }
            
            // Deselect after attempting move
            deselect_unit(&mut unit_selection, &mut commands);
        }
    }
}

fn select_unit(
    unit_entity: Entity,
    unit_selection: &mut ResMut<UnitSelection>,
    unit_query: &mut Query<(Entity, &mut Unit), With<Unit>>,
    tile_query: &Query<&MapTile>,
    commands: &mut Commands,
) {
    // Deselect previous unit
    deselect_unit(unit_selection, commands);
    
    // Select new unit
    unit_selection.selected_unit = Some(unit_entity);
    
    if let Ok((_, unit)) = unit_query.get(unit_entity) {
        // Calculate valid moves
        unit_selection.valid_moves = unit.calculate_valid_moves(tile_query);
        
        // Create movement indicators
        let valid_moves_copy = unit_selection.valid_moves.clone();
        for &move_coord in &valid_moves_copy {
            let world_pos = move_coord.to_world_pos(super::map::HEX_SIZE);
            let indicator = commands.spawn((
                MovementIndicator,
                Text2d::new("○"), // Circle outline for valid moves
                TextColor(Color::srgb(0.0, 1.0, 0.0)), // Green
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                Transform::from_translation(Vec3::new(world_pos.x, world_pos.y, 1.5)),
            )).id();
            
            unit_selection.movement_indicators.push(indicator);
        }
    }
}

fn deselect_unit(
    unit_selection: &mut ResMut<UnitSelection>,
    commands: &mut Commands,
) {
    unit_selection.selected_unit = None;
    unit_selection.valid_moves.clear();
    
    // Remove movement indicators
    for indicator_entity in unit_selection.movement_indicators.drain(..) {
        commands.entity(indicator_entity).despawn();
    }
}

// System for starting unit turns
pub fn start_unit_turns(
    mut unit_query: Query<&mut Unit>,
    civ_manager: Res<CivilizationManager>,
) {
    for mut unit in unit_query.iter_mut() {
        if civ_manager.is_current_turn(unit.civilization_id) {
            unit.start_turn();
        }
    }
}

// System for spawning unit markers (visual representation)
pub fn spawn_unit_markers(
    mut commands: Commands,
    units_query: Query<&Unit, Added<Unit>>,
    civ_manager: Res<CivilizationManager>,
) {
    for unit in units_query.iter() {
        let world_pos = unit.hex_coord.to_world_pos(super::map::HEX_SIZE);
        
        // Get civilization color
        let color = civ_manager.get_civilization(unit.civilization_id)
            .map(|civ| civ.color)
            .unwrap_or(Color::WHITE);
        
        // Create unit marker
        commands.spawn((
            UnitMarker {
                civilization_id: unit.civilization_id,
                unit_type: unit.unit_type,
            },
            Text2d::new(unit.unit_type.get_symbol()),
            TextColor(color),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            Transform::from_translation(Vec3::new(world_pos.x, world_pos.y, 3.0)), // Above cities
        ));
    }
}

// System for updating unit marker positions when units move
pub fn update_unit_marker_positions(
    unit_query: Query<&Unit, Changed<Unit>>,
    mut marker_query: Query<(&UnitMarker, &mut Transform)>,
) {
    for unit in unit_query.iter() {
        // Find the corresponding marker
        for (marker, mut transform) in marker_query.iter_mut() {
            if marker.civilization_id == unit.civilization_id && marker.unit_type == unit.unit_type {
                let world_pos = unit.hex_coord.to_world_pos(super::map::HEX_SIZE);
                transform.translation = Vec3::new(world_pos.x, world_pos.y, 3.0);
                break; // Assume one marker per unit for now
            }
        }
    }
}

// Function to spawn a unit
pub fn spawn_unit(
    commands: &mut Commands,
    unit_type: UnitType,
    civilization_id: u32,
    hex_coord: HexCoord,
    civ_manager: &mut CivilizationManager,
) -> Entity {
    let unit = Unit::new(unit_type, civilization_id, hex_coord);
    let unit_entity = commands.spawn(unit).id();
    
    // Add unit to civilization
    if let Some(civ) = civ_manager.get_civilization_mut(civilization_id) {
        civ.add_unit(unit_entity);
    }
    
    unit_entity
}

// Function to spawn a city
pub fn spawn_city(
    commands: &mut Commands,
    name: String,
    hex_coord: HexCoord,
    civilization_id: u32,
    turn: u32,
    is_capital: bool,
    civ_manager: &mut CivilizationManager,
) -> Entity {
    let city = City::new(name, hex_coord, civilization_id, turn, is_capital);
    let city_entity = commands.spawn(city).id();
    
    // Add city to civilization
    if let Some(civ) = civ_manager.get_civilization_mut(civilization_id) {
        civ.add_city(city_entity);
    }
    
    city_entity
}