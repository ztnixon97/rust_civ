use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct CivilizationManager {
    pub civilizations: HashMap<u32, Civilization>,
    pub next_civ_id: u32,
    pub current_turn_civ: u32,
    pub turn_number: u32,
}

impl Default for CivilizationManager {
    fn default() -> Self {
        Self {
            civilizations: HashMap::new(),
            next_civ_id: 1, // Start at 1, 0 is reserved for neutral/barbarian
            current_turn_civ: 1,
            turn_number: 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Civilization {
    pub id: u32,
    pub name: String,
    pub leader_name: String,
    pub color: Color,
    pub is_player: bool,
    pub is_ai: bool,
    pub civ_type: CivilizationType,
    pub traits: Vec<CivTrait>,
    pub cities: Vec<Entity>,
    pub units: Vec<Entity>,
    pub technologies: Vec<Technology>,
    pub culture: f32,
    pub science_points: f32,
    pub gold: f32,
    pub military_strength: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CivilizationType {
    Agricultural,  // Bonus to food and growth
    Commercial,    // Bonus to trade and gold
    Military,      // Bonus to unit production and combat
    Scientific,    // Bonus to research and technology
    Cultural,      // Bonus to culture and territorial expansion
    Maritime,      // Bonus to naval units and coastal settlements
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CivTrait {
    Expansionist,      // Faster settler production, movement bonus
    Industrious,       // Production bonus to buildings and units
    Commercial,        // Extra trade routes, gold bonus
    Militaristic,      // Combat bonuses, cheaper military units
    Scientific,        // Research bonus, starts with extra tech
    Seafaring,         // Naval bonuses, can cross oceans earlier
    Spiritual,         // Culture bonus, faster border expansion
    Organized,         // Maintenance cost reduction, efficiency bonus
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Technology {
    // Ancient Era
    Agriculture,
    AnimalHusbandry,
    Mining,
    Pottery,
    TheWheel,
    Writing,
    Archery,
    Masonry,
    
    // Classical Era
    Mathematics,
    Currency,
    Ironworking,
    Construction,
    HorsebackRiding,
    
    // More can be added as needed
}

impl Civilization {
    pub fn new(id: u32, name: String, leader_name: String, color: Color, civ_type: CivilizationType, is_player: bool) -> Self {
        let traits = Self::get_default_traits(civ_type);
        let starting_techs = Self::get_starting_technologies(civ_type);
        
        Self {
            id,
            name,
            leader_name,
            color,
            is_player,
            is_ai: !is_player,
            civ_type,
            traits,
            cities: Vec::new(),
            units: Vec::new(),
            technologies: starting_techs,
            culture: 0.0,
            science_points: 0.0,
            gold: 50.0, // Starting gold
            military_strength: 0.0,
        }
    }
    
    fn get_default_traits(civ_type: CivilizationType) -> Vec<CivTrait> {
        match civ_type {
            CivilizationType::Agricultural => vec![CivTrait::Expansionist, CivTrait::Organized],
            CivilizationType::Commercial => vec![CivTrait::Commercial, CivTrait::Seafaring],
            CivilizationType::Military => vec![CivTrait::Militaristic, CivTrait::Organized],
            CivilizationType::Scientific => vec![CivTrait::Scientific, CivTrait::Industrious],
            CivilizationType::Cultural => vec![CivTrait::Spiritual, CivTrait::Expansionist],
            CivilizationType::Maritime => vec![CivTrait::Seafaring, CivTrait::Commercial],
        }
    }
    
    fn get_starting_technologies(civ_type: CivilizationType) -> Vec<Technology> {
        let mut techs = vec![Technology::Agriculture]; // Everyone starts with agriculture
        
        match civ_type {
            CivilizationType::Agricultural => techs.push(Technology::AnimalHusbandry),
            CivilizationType::Commercial => techs.push(Technology::Pottery),
            CivilizationType::Military => techs.push(Technology::Archery),
            CivilizationType::Scientific => techs.push(Technology::Writing),
            CivilizationType::Cultural => techs.push(Technology::Masonry),
            CivilizationType::Maritime => techs.push(Technology::Pottery),
        }
        
        techs
    }
    
    pub fn has_technology(&self, tech: Technology) -> bool {
        self.technologies.contains(&tech)
    }
    
    pub fn add_city(&mut self, city_entity: Entity) {
        self.cities.push(city_entity);
    }
    
    pub fn add_unit(&mut self, unit_entity: Entity) {
        self.units.push(unit_entity);
    }
    
    pub fn remove_city(&mut self, city_entity: Entity) {
        self.cities.retain(|&e| e != city_entity);
    }
    
    pub fn remove_unit(&mut self, unit_entity: Entity) {
        self.units.retain(|&e| e != unit_entity);
    }
    
    pub fn get_trait_bonus(&self, trait_type: CivTrait) -> f32 {
        if self.traits.contains(&trait_type) {
            match trait_type {
                CivTrait::Expansionist => 1.5,      // 50% bonus
                CivTrait::Industrious => 1.25,      // 25% bonus
                CivTrait::Commercial => 1.3,        // 30% bonus
                CivTrait::Militaristic => 1.2,      // 20% bonus
                CivTrait::Scientific => 1.4,        // 40% bonus
                CivTrait::Seafaring => 1.3,         // 30% bonus
                CivTrait::Spiritual => 1.35,        // 35% bonus
                CivTrait::Organized => 0.8,         // 20% cost reduction (multiplicative)
            }
        } else {
            1.0
        }
    }
}

impl CivilizationManager {
    pub fn add_civilization(&mut self, mut civ: Civilization) -> u32 {
        let id = self.next_civ_id;
        civ.id = id;
        self.civilizations.insert(id, civ);
        self.next_civ_id += 1;
        id
    }
    
    pub fn get_civilization(&self, id: u32) -> Option<&Civilization> {
        self.civilizations.get(&id)
    }
    
    pub fn get_civilization_mut(&mut self, id: u32) -> Option<&mut Civilization> {
        self.civilizations.get_mut(&id)
    }
    
    pub fn get_player_civilization(&self) -> Option<&Civilization> {
        self.civilizations.values().find(|civ| civ.is_player)
    }
    
    pub fn get_player_civilization_mut(&mut self) -> Option<&mut Civilization> {
        self.civilizations.values_mut().find(|civ| civ.is_player)
    }
    
    pub fn next_turn(&mut self) {
        // Advance to next civilization's turn
        let civ_ids: Vec<u32> = self.civilizations.keys().cloned().collect();
        if let Some(current_index) = civ_ids.iter().position(|&id| id == self.current_turn_civ) {
            let next_index = (current_index + 1) % civ_ids.len();
            self.current_turn_civ = civ_ids[next_index];
            
            // If we're back to the first civ, increment turn number
            if next_index == 0 {
                self.turn_number += 1;
            }
        }
    }
    
    pub fn is_current_turn(&self, civ_id: u32) -> bool {
        self.current_turn_civ == civ_id
    }
}

// Predefined civilizations for easy setup
pub fn create_default_civilizations() -> Vec<Civilization> {
    vec![
        Civilization::new(
            0, // Will be assigned by manager
            "Roman Empire".to_string(),
            "Caesar Augustus".to_string(),
            Color::srgb(0.8, 0.1, 0.1), // Red
            CivilizationType::Military,
            true, // Player civilization
        ),
        Civilization::new(
            0,
            "Egyptian Kingdom".to_string(),
            "Cleopatra".to_string(),
            Color::srgb(0.9, 0.8, 0.1), // Yellow/Gold
            CivilizationType::Cultural,
            false,
        ),
        Civilization::new(
            0,
            "Greek City-States".to_string(),
            "Pericles".to_string(),
            Color::srgb(0.1, 0.3, 0.8), // Blue
            CivilizationType::Scientific,
            false,
        ),
        Civilization::new(
            0,
            "Phoenician Traders".to_string(),
            "Hiram".to_string(),
            Color::srgb(0.6, 0.2, 0.8), // Purple
            CivilizationType::Maritime,
            false,
        ),
        Civilization::new(
            0,
            "Celtic Tribes".to_string(),
            "Vercingetorix".to_string(),
            Color::srgb(0.1, 0.7, 0.2), // Green
            CivilizationType::Agricultural,
            false,
        ),
        Civilization::new(
            0,
            "Mesopotamian Empire".to_string(),
            "Hammurabi".to_string(),
            Color::srgb(0.8, 0.4, 0.1), // Orange/Brown
            CivilizationType::Commercial,
            false,
        ),
    ]
}