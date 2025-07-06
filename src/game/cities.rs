use bevy::prelude::*;
use super::hex::HexCoord;
use super::map::{MapTile, TerrainType};
use super::civilization::{CivilizationManager, CivTrait};
use std::collections::HashMap;

#[derive(Component)]
pub struct City {
    pub name: String,
    pub hex_coord: HexCoord,
    pub civilization_id: u32,
    pub population: u32,
    pub founded_turn: u32,
    
    // Core yields
    pub food_per_turn: f32,
    pub production_per_turn: f32,
    pub science_per_turn: f32,
    pub gold_per_turn: f32,
    pub culture_per_turn: f32,
    
    // Growth and development
    pub food_stored: f32,
    pub food_needed_for_growth: f32,
    pub culture_stored: f32,
    pub culture_needed_for_expansion: f32,
    
    // Territory and worked tiles
    pub territory_tiles: Vec<HexCoord>,     // All tiles in city's influence
    pub worked_tiles: Vec<HexCoord>,        // Tiles currently being worked by population
    pub territory_radius: u32,              // How far the territory extends
    
    // Buildings and improvements
    pub buildings: Vec<Building>,
    pub production_queue: Vec<ProductionItem>,
    pub current_production: Option<ProductionItem>,
    pub production_progress: f32,
    
    // City status
    pub is_capital: bool,
    pub happiness: f32,
    pub health: f32,
    pub defense_strength: f32,
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Building {
    Granary,        // +2 food, 25% food storage bonus
    Barracks,       // +2 military unit experience, +1 defense
    Library,        // +2 science, +25% science in city
    Marketplace,    // +2 gold, +25% gold in city
    Temple,         // +2 culture, +1 happiness
    Walls,          // +3 defense, +50% defense against attacks
    Aqueduct,       // +2 health, allows city growth beyond size 6
    Workshop,       // +1 production, +25% production for buildings
    Harbor,         // +1 food, +2 gold from water tiles (coastal cities only)
    Lighthouse,     // +1 food from water tiles, +2 trade routes (coastal cities only)
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProductionItem {
    Building(Building),
    Unit(UnitType),
    Wonder(Wonder),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnitType {
    Warrior,
    Archer,
    Spearman,
    Settler,
    Worker,
    Scout,
    Galley,      // Basic naval unit
    Trireme,     // Advanced naval unit
}

#[derive(Clone, Debug, PartialEq)]
pub enum Wonder {
    Pyramids,       // +2 culture, free granary in every city
    Stonehenge,     // +4 culture, +1 culture from temples
    Colossus,       // +3 gold, +1 gold from water tiles
    GreatLibrary,   // +4 science, free library in every city
}

#[derive(Component)]
pub struct CityMarker {
    pub civilization_id: u32,
    pub city_name: String,
}

impl City {
    pub fn new(name: String, hex_coord: HexCoord, civilization_id: u32, turn: u32, is_capital: bool) -> Self {
        let initial_territory = Self::calculate_initial_territory(hex_coord);
        let food_needed = Self::calculate_food_needed_for_growth(1);
        let culture_needed = Self::calculate_culture_needed_for_expansion(1);
        
        Self {
            name,
            hex_coord,
            civilization_id,
            population: 1,
            founded_turn: turn,
            food_per_turn: 2.0,      // Base food from city center
            production_per_turn: 1.0, // Base production
            science_per_turn: 1.0,    // Base science
            gold_per_turn: 2.0,       // Base gold
            culture_per_turn: 1.0,    // Base culture
            food_stored: 0.0,
            food_needed_for_growth: food_needed,
            culture_stored: 0.0,
            culture_needed_for_expansion: culture_needed,
            territory_tiles: initial_territory,
            worked_tiles: vec![hex_coord], // Start by working the city center
            territory_radius: 1,
            buildings: Vec::new(),
            production_queue: Vec::new(),
            current_production: None,
            production_progress: 0.0,
            is_capital,
            happiness: 5.0,           // Base happiness
            health: 5.0,              // Base health
            defense_strength: 2.0,    // Base defense
        }
    }
    
    fn calculate_initial_territory(center: HexCoord) -> Vec<HexCoord> {
        let mut territory = vec![center]; // City center
        
        // Add immediate neighbors (6 tiles around city)
        for neighbor in center.neighbors() {
            territory.push(neighbor);
        }
        
        territory
    }
    
    fn calculate_food_needed_for_growth(population: u32) -> f32 {
        // Formula: 10 + (population * 2)
        10.0 + (population as f32 * 2.0)
    }
    
    fn calculate_culture_needed_for_expansion(territory_radius: u32) -> f32 {
        // Formula: 20 * (radius^2)
        20.0 * (territory_radius as f32).powi(2)
    }
    
    pub fn calculate_yields(&mut self, tile_query: &Query<&MapTile>, civ_manager: &CivilizationManager) {
        let mut total_food = 0.0;
        let mut total_production = 0.0;
        let mut total_science = 0.0;
        let mut total_gold = 0.0;
        let mut total_culture = 1.0; // Base culture
        
        // Get civilization bonuses
        let civ_bonuses = if let Some(civ) = civ_manager.get_civilization(self.civilization_id) {
            (
                civ.get_trait_bonus(CivTrait::Expansionist),
                civ.get_trait_bonus(CivTrait::Commercial),
                civ.get_trait_bonus(CivTrait::Scientific),
                civ.get_trait_bonus(CivTrait::Spiritual),
            )
        } else {
            (1.0, 1.0, 1.0, 1.0)
        };
        
        // Calculate yields from worked tiles
        for &tile_coord in &self.worked_tiles {
            if let Some(tile) = tile_query.iter().find(|t| t.hex_coord == tile_coord) {
                let (food, production, science) = self.get_tile_yields(tile);
                total_food += food;
                total_production += production;
                total_science += science;
                total_gold += self.get_tile_gold_yield(tile);
            }
        }
        
        // Apply building bonuses
        for building in &self.buildings {
            let (food_bonus, prod_bonus, sci_bonus, gold_bonus, culture_bonus) = building.get_yields();
            total_food += food_bonus;
            total_production += prod_bonus;
            total_science += sci_bonus;
            total_gold += gold_bonus;
            total_culture += culture_bonus;
        }
        
        // Apply civilization trait bonuses
        total_gold *= civ_bonuses.1;      // Commercial bonus
        total_science *= civ_bonuses.2;   // Scientific bonus
        total_culture *= civ_bonuses.3;   // Spiritual bonus
        
        // Store calculated yields
        self.food_per_turn = total_food;
        self.production_per_turn = total_production;
        self.science_per_turn = total_science;
        self.gold_per_turn = total_gold;
        self.culture_per_turn = total_culture;
    }
    
    fn get_tile_yields(&self, tile: &MapTile) -> (f32, f32, f32) {
        let terrain = TerrainType::from_u8(tile.terrain);
        let (mut food, mut production, mut science) = terrain.base_yields();
        
        // Resource bonuses
        if tile.resource != 0 {
            match tile.resource {
                3 => food += 2.0,        // Wheat
                4 => food += 2.0,        // Fish
                6 => production += 2.0,  // Wood
                2 | 5 => production += 1.0, // Iron, Stone
                _ => {}
            }
        }
        
        // River bonus
        if tile.has_river {
            food += 1.0;
        }
        
        // Fertility bonus
        food += tile.soil_fertility * 2.0;
        
        (food, production, science)
    }
    
    fn get_tile_gold_yield(&self, tile: &MapTile) -> f32 {
        let mut gold = 0.0;
        
        // Trade value from tile
        gold += tile.trade_value * 2.0;
        
        // Coastal bonus
        if tile.is_coastal {
            gold += 1.0;
        }
        
        // Resource bonuses
        if tile.resource != 0 {
            match tile.resource {
                1 => gold += 3.0,  // Gold
                9 => gold += 2.0,  // Gems
                13 => gold += 1.0, // Spices
                14 => gold += 1.0, // Silk
                _ => {}
            }
        }
        
        gold
    }
    
    pub fn process_turn(&mut self, civ_manager: &mut CivilizationManager) {
        // Add food and check for growth
        self.food_stored += self.food_per_turn;
        if self.food_stored >= self.food_needed_for_growth {
            self.grow_population();
        }
        
        // Add culture and check for territory expansion
        self.culture_stored += self.culture_per_turn;
        if self.culture_stored >= self.culture_needed_for_expansion {
            self.expand_territory();
        }
        
        // Process production
        if let Some(ref production_item) = self.current_production.clone() {
            self.production_progress += self.production_per_turn;
            
            let required_production = production_item.get_required_production();
            if self.production_progress >= required_production {
                self.complete_production(civ_manager);
            }
        } else {
            // Auto-assign production if queue has items
            if !self.production_queue.is_empty() {
                self.current_production = Some(self.production_queue.remove(0));
                self.production_progress = 0.0;
            }
        }
        
        // Update civilization totals
        if let Some(civ) = civ_manager.get_civilization_mut(self.civilization_id) {
            civ.science_points += self.science_per_turn;
            civ.gold += self.gold_per_turn;
            civ.culture += self.culture_per_turn;
        }
    }
    
    fn grow_population(&mut self) {
        self.population += 1;
        self.food_stored = 0.0;
        self.food_needed_for_growth = Self::calculate_food_needed_for_growth(self.population);
        
        // Can work one more tile
        self.assign_best_available_tile();
        
        println!("City {} has grown to population {}!", self.name, self.population);
    }
    
    fn expand_territory(&mut self) {
        self.territory_radius += 1;
        self.culture_stored = 0.0;
        self.culture_needed_for_expansion = Self::calculate_culture_needed_for_expansion(self.territory_radius);
        
        // Add new tiles to territory (simplified - would need better algorithm)
        let new_tiles = self.calculate_territory_expansion();
        self.territory_tiles.extend(new_tiles);
        
        println!("City {} has expanded its territory! (Radius: {})", self.name, self.territory_radius);
    }
    
    fn calculate_territory_expansion(&self) -> Vec<HexCoord> {
        // This is a simplified expansion - in a full game you'd want more sophisticated territory calculation
        let mut new_tiles = Vec::new();
        
        // Add tiles at the new radius distance
        for existing_tile in &self.territory_tiles {
            for neighbor in existing_tile.neighbors() {
                if !self.territory_tiles.contains(&neighbor) && !new_tiles.contains(&neighbor) {
                    let distance = self.hex_distance(neighbor, self.hex_coord);
                    if distance <= self.territory_radius as i32 {
                        new_tiles.push(neighbor);
                    }
                }
            }
        }
        
        new_tiles
    }
    
    fn assign_best_available_tile(&mut self) {
        // Find the best unworked tile in territory
        let mut best_tile = None;
        let mut best_value = 0.0;
        
        for &tile_coord in &self.territory_tiles {
            if !self.worked_tiles.contains(&tile_coord) {
                // Would need tile_query access here - simplified for now
                let estimated_value = 3.0; // Placeholder
                if estimated_value > best_value {
                    best_value = estimated_value;
                    best_tile = Some(tile_coord);
                }
            }
        }
        
        if let Some(tile) = best_tile {
            self.worked_tiles.push(tile);
        }
    }
    
    fn complete_production(&mut self, civ_manager: &mut CivilizationManager) {
        if let Some(item) = self.current_production.take() {
            match item {
                ProductionItem::Building(building) => {
                    self.buildings.push(building);
                    println!("City {} completed building: {:?}", self.name, building);
                }
                ProductionItem::Unit(unit_type) => {
                    // Would spawn unit entity here
                    println!("City {} completed unit: {:?}", self.name, unit_type);
                }
                ProductionItem::Wonder(wonder) => {
                    // Apply wonder effects
                    println!("City {} completed wonder: {:?}", self.name, wonder);
                }
            }
            
            self.production_progress = 0.0;
            
            // Start next item in queue if available
            if !self.production_queue.is_empty() {
                self.current_production = Some(self.production_queue.remove(0));
            }
        }
    }
    
    fn hex_distance(&self, a: HexCoord, b: HexCoord) -> i32 {
        let dq = (a.q - b.q).abs();
        let dr = (a.r - b.r).abs();
        let ds = (-(a.q + a.r) + (b.q + b.r)).abs();
        (dq.max(dr)).max(ds)
    }
    
    pub fn can_build(&self, item: &ProductionItem) -> bool {
        match item {
            ProductionItem::Building(building) => {
                !self.buildings.contains(building) && self.meets_building_requirements(building)
            }
            ProductionItem::Unit(_) => true, // Can always build units if you have resources
            ProductionItem::Wonder(wonder) => self.meets_wonder_requirements(wonder),
        }
    }
    
    fn meets_building_requirements(&self, building: &Building) -> bool {
        match building {
            Building::Harbor | Building::Lighthouse => {
                // Requires coastal city - would need tile query to check
                true // Simplified
            }
            Building::Aqueduct => self.population >= 4,
            _ => true,
        }
    }
    
    fn meets_wonder_requirements(&self, _wonder: &Wonder) -> bool {
        // Would check for required technologies, resources, etc.
        true // Simplified
    }
}

impl Building {
    pub fn get_yields(&self) -> (f32, f32, f32, f32, f32) {
        // Returns (food, production, science, gold, culture)
        match self {
            Building::Granary => (2.0, 0.0, 0.0, 0.0, 0.0),
            Building::Barracks => (0.0, 0.0, 0.0, 0.0, 0.0),
            Building::Library => (0.0, 0.0, 2.0, 0.0, 0.0),
            Building::Marketplace => (0.0, 0.0, 0.0, 2.0, 0.0),
            Building::Temple => (0.0, 0.0, 0.0, 0.0, 2.0),
            Building::Walls => (0.0, 0.0, 0.0, 0.0, 0.0),
            Building::Aqueduct => (0.0, 0.0, 0.0, 0.0, 0.0),
            Building::Workshop => (0.0, 1.0, 0.0, 0.0, 0.0),
            Building::Harbor => (1.0, 0.0, 0.0, 2.0, 0.0),
            Building::Lighthouse => (1.0, 0.0, 0.0, 0.0, 0.0),
        }
    }
    
    pub fn get_name(&self) -> &'static str {
        match self {
            Building::Granary => "Granary",
            Building::Barracks => "Barracks",
            Building::Library => "Library",
            Building::Marketplace => "Marketplace",
            Building::Temple => "Temple",
            Building::Walls => "Walls",
            Building::Aqueduct => "Aqueduct",
            Building::Workshop => "Workshop",
            Building::Harbor => "Harbor",
            Building::Lighthouse => "Lighthouse",
        }
    }
}

impl ProductionItem {
    pub fn get_required_production(&self) -> f32 {
        match self {
            ProductionItem::Building(building) => match building {
                Building::Granary => 60.0,
                Building::Barracks => 60.0,
                Building::Library => 90.0,
                Building::Marketplace => 100.0,
                Building::Temple => 80.0,
                Building::Walls => 100.0,
                Building::Aqueduct => 120.0,
                Building::Workshop => 120.0,
                Building::Harbor => 100.0,
                Building::Lighthouse => 80.0,
            },
            ProductionItem::Unit(unit) => match unit {
                UnitType::Warrior => 15.0,
                UnitType::Archer => 25.0,
                UnitType::Spearman => 35.0,
                UnitType::Settler => 100.0,
                UnitType::Worker => 60.0,
                UnitType::Scout => 15.0,
                UnitType::Galley => 40.0,
                UnitType::Trireme => 60.0,
            },
            ProductionItem::Wonder(wonder) => match wonder {
                Wonder::Pyramids => 400.0,
                Wonder::Stonehenge => 300.0,
                Wonder::Colossus => 350.0,
                Wonder::GreatLibrary => 400.0,
            },
        }
    }
    
    pub fn get_name(&self) -> String {
        match self {
            ProductionItem::Building(building) => building.get_name().to_string(),
            ProductionItem::Unit(unit) => format!("{:?}", unit),
            ProductionItem::Wonder(wonder) => format!("{:?}", wonder),
        }
    }
}

// System for processing city turns
pub fn process_city_turns(
    mut city_query: Query<&mut City>,
    tile_query: Query<&MapTile>,
    mut civ_manager: ResMut<CivilizationManager>,
) {
    for mut city in city_query.iter_mut() {
        // Only process cities for the current civilization's turn
        if civ_manager.is_current_turn(city.civilization_id) {
            city.calculate_yields(&tile_query, &civ_manager);
            city.process_turn(&mut civ_manager);
        }
    }
}

// System for spawning city markers (visual representation)
pub fn spawn_city_markers(
    mut commands: Commands,
    cities_query: Query<&City, Added<City>>,
    civ_manager: Res<CivilizationManager>,
) {
    for city in cities_query.iter() {
        let world_pos = city.hex_coord.to_world_pos(super::map::HEX_SIZE);
        
        // Get civilization color
        let color = civ_manager.get_civilization(city.civilization_id)
            .map(|civ| civ.color)
            .unwrap_or(Color::WHITE);
        
        // Create city marker
        commands.spawn((
            CityMarker {
                civilization_id: city.civilization_id,
                city_name: city.name.clone(),
            },
            Text2d::new("●"), // Circle symbol for city
            TextColor(color),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            Transform::from_translation(Vec3::new(world_pos.x, world_pos.y, 2.0)), // Above tiles
        ));
        
        // Add city name text below the marker
        commands.spawn((
            Text2d::new(city.name.clone()),
            TextColor(Color::WHITE),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            Transform::from_translation(Vec3::new(world_pos.x, world_pos.y - 20.0, 2.0)),
        ));
    }
}