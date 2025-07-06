use bevy::prelude::*;
use super::hex::HexCoord;
use super::world_gen::BiomeType;
use noise::{NoiseFn, Perlin};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ResourceType {
    None = 0,
    Gold = 1,
    Iron = 2,
    Wheat = 3,
    Fish = 4,
    Stone = 5,
    Wood = 6,
    Oil = 7,
    Horses = 8,
    Gems = 9,
    Copper = 10,
    Coal = 11,
    Cattle = 12,
    Spices = 13,
    Silk = 14,
    Wine = 15,
    Salt = 16,
}

impl ResourceType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => ResourceType::Gold,
            2 => ResourceType::Iron,
            3 => ResourceType::Wheat,
            4 => ResourceType::Fish,
            5 => ResourceType::Stone,
            6 => ResourceType::Wood,
            7 => ResourceType::Oil,
            8 => ResourceType::Horses,
            9 => ResourceType::Gems,
            10 => ResourceType::Copper,
            11 => ResourceType::Coal,
            12 => ResourceType::Cattle,
            13 => ResourceType::Spices,
            14 => ResourceType::Silk,
            15 => ResourceType::Wine,
            16 => ResourceType::Salt,
            _ => ResourceType::None,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            ResourceType::None => "",
            ResourceType::Gold => "$",
            ResourceType::Iron => "#",
            ResourceType::Wheat => "*",
            ResourceType::Fish => "~",
            ResourceType::Stone => "o",
            ResourceType::Wood => "T",
            ResourceType::Oil => "O",
            ResourceType::Horses => "H",
            ResourceType::Gems => "^",
            ResourceType::Copper => "c",
            ResourceType::Coal => "@",
            ResourceType::Cattle => "C",
            ResourceType::Spices => "s",
            ResourceType::Silk => "S",
            ResourceType::Wine => "w",
            ResourceType::Salt => "=",
        }
    }

    pub fn color(self) -> Color {
        match self {
            ResourceType::None => Color::WHITE,
            ResourceType::Gold => Color::srgb(1.0, 0.8, 0.0),
            ResourceType::Iron => Color::srgb(0.6, 0.6, 0.6),
            ResourceType::Wheat => Color::srgb(1.0, 1.0, 0.0),
            ResourceType::Fish => Color::srgb(0.0, 0.8, 1.0),
            ResourceType::Stone => Color::srgb(0.8, 0.8, 0.8),
            ResourceType::Wood => Color::srgb(0.0, 0.8, 0.0),
            ResourceType::Oil => Color::srgb(0.2, 0.2, 0.2),
            ResourceType::Horses => Color::srgb(0.8, 0.6, 0.4),
            ResourceType::Gems => Color::srgb(1.0, 0.0, 1.0),
            ResourceType::Copper => Color::srgb(0.8, 0.4, 0.2),
            ResourceType::Coal => Color::srgb(0.3, 0.3, 0.3),
            ResourceType::Cattle => Color::srgb(0.6, 0.4, 0.2),
            ResourceType::Spices => Color::srgb(1.0, 0.5, 0.0),
            ResourceType::Silk => Color::srgb(1.0, 1.0, 1.0),
            ResourceType::Wine => Color::srgb(0.6, 0.2, 0.6),
            ResourceType::Salt => Color::srgb(0.9, 0.9, 0.9),
        }
    }

    /// Get resources that can appear on specific biome types
    pub fn for_biome(biome: u8) -> Vec<ResourceType> {
        match BiomeType::from_u8(biome) {
            BiomeType::Ocean | BiomeType::Lake | BiomeType::River => {
                vec![ResourceType::Fish]
            },
            BiomeType::TemperateGrassland | BiomeType::TropicalGrasslandSavanna => {
                vec![ResourceType::Wheat, ResourceType::Horses, ResourceType::Cattle]
            },
            BiomeType::AlpineTundra | BiomeType::MontaneForest => {
                vec![ResourceType::Gold, ResourceType::Gems, ResourceType::Stone, ResourceType::Iron]
            },
            BiomeType::TemperateDeciduousForest | BiomeType::TemperateConiferForest | 
            BiomeType::TaigaBorealForest => {
                vec![ResourceType::Wood, ResourceType::Iron, ResourceType::Copper]
            },
            BiomeType::TropicalRainforest | BiomeType::TropicalSeasonalForest => {
                vec![ResourceType::Wood, ResourceType::Spices, ResourceType::Silk]
            },
            BiomeType::HotDesert => {
                vec![ResourceType::Oil, ResourceType::Gold, ResourceType::Gems]
            },
            BiomeType::ColdDesert => {
                vec![ResourceType::Oil, ResourceType::Iron, ResourceType::Coal]
            },
            BiomeType::TundraBarren | BiomeType::TundraWet => {
                vec![ResourceType::Oil, ResourceType::Iron]
            },
            BiomeType::Shrubland => {
                vec![ResourceType::Stone, ResourceType::Copper]
            },
            BiomeType::TemperateRainforest => {
                vec![ResourceType::Wood, ResourceType::Wine]
            },
            BiomeType::Mangrove => {
                vec![ResourceType::Fish, ResourceType::Wood]
            },
            BiomeType::SaltMarsh => {
                vec![ResourceType::Fish, ResourceType::Salt]
            },
            BiomeType::Wetland => {
                vec![ResourceType::Fish, ResourceType::Cattle]
            },
        }
    }
}

pub fn generate_resource(hex_coord: HexCoord, biome: u8) -> u8 {
    let resource_noise = Perlin::new(789);
    
    // Lower chance of resources (about 15% of tiles)
    let resource_chance = resource_noise.get([
        hex_coord.q as f64 * 0.3,
        hex_coord.r as f64 * 0.3,
    ]) as f32;
    
    if resource_chance > 0.7 {
        let possible_resources = ResourceType::for_biome(biome);
        if !possible_resources.is_empty() {
            // Use coordinate hash to pick consistent resource
            let index = ((hex_coord.q.abs() + hex_coord.r.abs() * 3) as usize) % possible_resources.len();
            return possible_resources[index] as u8;
        }
    }
    
    ResourceType::None as u8
}

#[derive(Component)]
pub struct ResourceMarker {
    pub resource_type: ResourceType,
}

#[derive(Component)]
pub struct RiverMarker;

pub fn spawn_resource_markers(
    mut commands: Commands,
    tiles_query: Query<(Entity, &crate::game::map::MapTile, &Transform), Added<crate::game::map::MapTile>>,
) {
    for (tile_entity, tile, _transform) in tiles_query.iter() {
        let mut children = Vec::new();
        
        // Add resource marker if tile has a resource
        if tile.resource != 0 {
            let resource_type = ResourceType::from_u8(tile.resource);
            
            let resource_marker = commands.spawn((
                ResourceMarker { resource_type },
                Text2d::new(resource_type.symbol()),
                TextColor(resource_type.color()),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                Transform::from_translation(Vec3::new(8.0, 8.0, 1.0)), // Top-right corner
            )).id();
            
            children.push(resource_marker);
        }
        
        // Add river marker if tile has a river
        if tile.has_river {
            let river_marker = commands.spawn((
                RiverMarker,
                Text2d::new("≈"), // Wave symbol for river
                TextColor(Color::srgb(0.3, 0.6, 1.0)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                Transform::from_translation(Vec3::new(-8.0, -8.0, 1.0)), // Bottom-left corner
            )).id();
            
            children.push(river_marker);
        }
        
        // Make markers children of the tile
        if !children.is_empty() {
            commands.entity(tile_entity).add_children(&children);
        }
    }
}