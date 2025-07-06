use bevy::prelude::*;
use super::hex::HexCoord;
use super::world_gen::{WorldGenerator, BiomeType, WorldGenConfig, WorldTile, StrategicFeature};
use std::f32::consts::PI;
use std::collections::HashMap;

pub const HEX_SIZE: f32 = 30.0;
pub const MAP_RADIUS: i32 = 100;

#[derive(Component)]
pub struct MapTile {
    pub hex_coord: HexCoord,
    pub terrain: u8,
    pub biome: u8,
    pub elevation: u8,              // Stored as 0-255 for compatibility
    pub elevation_raw: f32,         // Keep raw elevation for calculations
    pub resource: u8,
    pub has_river: bool,
    pub river_flow: f32,
    pub is_coastal: bool,
    pub water_distance: u8,
    pub temperature: f32,           // 0.0 to 1.0
    pub precipitation: f32,         // 0.0 to 1.0
    pub soil_fertility: f32,        // 0.0 to 1.0
    pub geology: u8,
    
    // Strategic Geography
    pub strategic_feature: u8,      // Type of strategic feature
    pub defensibility: f32,         // 0.0 to 1.0
    pub trade_value: f32,           // 0.0 to 1.0
    pub flood_risk: f32,            // 0.0 to 1.0
    pub naval_access: f32,          // 0.0 to 1.0
}

// Keep the old TerrainType for compatibility, but map it to BiomeType
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TerrainType {
    Ocean = 0,
    Lake = 1,
    River = 2,
    TundraBarren = 10,
    TundraWet = 11,
    TaigaBorealForest = 12,
    TemperateGrassland = 20,
    TemperateDeciduousForest = 21,
    TemperateConiferForest = 22,
    TemperateRainforest = 23,
    TropicalGrasslandSavanna = 30,
    TropicalSeasonalForest = 31,
    TropicalRainforest = 32,
    ColdDesert = 40,
    HotDesert = 41,
    Shrubland = 42,
    AlpineTundra = 50,
    MontaneForest = 51,
    Mangrove = 60,
    SaltMarsh = 61,
    Wetland = 62,
}

impl TerrainType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => TerrainType::Ocean,
            1 => TerrainType::Lake,
            2 => TerrainType::River,
            10 => TerrainType::TundraBarren,
            11 => TerrainType::TundraWet,
            12 => TerrainType::TaigaBorealForest,
            20 => TerrainType::TemperateGrassland,
            21 => TerrainType::TemperateDeciduousForest,
            22 => TerrainType::TemperateConiferForest,
            23 => TerrainType::TemperateRainforest,
            30 => TerrainType::TropicalGrasslandSavanna,
            31 => TerrainType::TropicalSeasonalForest,
            32 => TerrainType::TropicalRainforest,
            40 => TerrainType::ColdDesert,
            41 => TerrainType::HotDesert,
            42 => TerrainType::Shrubland,
            50 => TerrainType::AlpineTundra,
            51 => TerrainType::MontaneForest,
            60 => TerrainType::Mangrove,
            61 => TerrainType::SaltMarsh,
            62 => TerrainType::Wetland,
            _ => TerrainType::TemperateGrassland,
        }
    }

    pub fn color(self) -> Color {
        // Use the same colors as BiomeType
        BiomeType::from_u8(self as u8).color()
    }

    pub fn symbol(self) -> &'static str {
        match self {
            TerrainType::Ocean => "≈",
            TerrainType::Lake => "○",
            TerrainType::River => "≋",
            TerrainType::TundraBarren => "*",
            TerrainType::TundraWet => "~",
            TerrainType::TaigaBorealForest => "♦",
            TerrainType::TemperateGrassland => ".",
            TerrainType::TemperateDeciduousForest => "♠",
            TerrainType::TemperateConiferForest => "♣",
            TerrainType::TemperateRainforest => "♠",
            TerrainType::TropicalGrasslandSavanna => ":",
            TerrainType::TropicalSeasonalForest => "♥",
            TerrainType::TropicalRainforest => "♦",
            TerrainType::ColdDesert => "░",
            TerrainType::HotDesert => "▓",
            TerrainType::Shrubland => "▒",
            TerrainType::AlpineTundra => "▲",
            TerrainType::MontaneForest => "▲",
            TerrainType::Mangrove => "♠",
            TerrainType::SaltMarsh => "≈",
            TerrainType::Wetland => "~",
        }
    }

    /// Get display name for UI
    pub fn name(self) -> &'static str {
        match self {
            TerrainType::Ocean => "Ocean",
            TerrainType::Lake => "Lake",
            TerrainType::River => "River",
            TerrainType::TundraBarren => "Barren Tundra",
            TerrainType::TundraWet => "Wet Tundra",
            TerrainType::TaigaBorealForest => "Taiga Forest",
            TerrainType::TemperateGrassland => "Grassland",
            TerrainType::TemperateDeciduousForest => "Deciduous Forest",
            TerrainType::TemperateConiferForest => "Conifer Forest",
            TerrainType::TemperateRainforest => "Temperate Rainforest",
            TerrainType::TropicalGrasslandSavanna => "Savanna",
            TerrainType::TropicalSeasonalForest => "Seasonal Forest",
            TerrainType::TropicalRainforest => "Tropical Rainforest",
            TerrainType::ColdDesert => "Cold Desert",
            TerrainType::HotDesert => "Desert",
            TerrainType::Shrubland => "Shrubland",
            TerrainType::AlpineTundra => "Alpine Tundra",
            TerrainType::MontaneForest => "Mountain Forest",
            TerrainType::Mangrove => "Mangrove",
            TerrainType::SaltMarsh => "Salt Marsh",
            TerrainType::Wetland => "Wetland",
        }
    }

    /// Get basic yield information for gameplay
    pub fn base_yields(self) -> (f32, f32, f32) { // (food, production, science)
        match self {
            TerrainType::Ocean => (2.0, 0.0, 0.0),
            TerrainType::Lake => (2.0, 0.0, 0.0),
            TerrainType::River => (2.0, 0.0, 1.0),
            
            TerrainType::TundraBarren => (0.0, 1.0, 0.0),
            TerrainType::TundraWet => (1.0, 0.0, 0.0),
            TerrainType::TaigaBorealForest => (1.0, 2.0, 0.0),
            
            TerrainType::TemperateGrassland => (3.0, 0.0, 0.0),
            TerrainType::TemperateDeciduousForest => (1.0, 2.0, 1.0),
            TerrainType::TemperateConiferForest => (1.0, 2.0, 0.0),
            TerrainType::TemperateRainforest => (1.0, 1.0, 2.0),
            
            TerrainType::TropicalGrasslandSavanna => (2.0, 0.0, 0.0),
            TerrainType::TropicalSeasonalForest => (2.0, 1.0, 1.0),
            TerrainType::TropicalRainforest => (1.0, 0.0, 3.0),
            
            TerrainType::ColdDesert => (0.0, 1.0, 0.0),
            TerrainType::HotDesert => (0.0, 0.0, 0.0),
            TerrainType::Shrubland => (1.0, 0.0, 0.0),
            
            TerrainType::AlpineTundra => (0.0, 1.0, 1.0),
            TerrainType::MontaneForest => (0.0, 2.0, 1.0),
            
            TerrainType::Mangrove => (2.0, 1.0, 1.0),
            TerrainType::SaltMarsh => (1.0, 0.0, 0.0),
            TerrainType::Wetland => (3.0, 0.0, 0.0),
        }
    }
}

#[derive(Resource)]
pub struct TerrainAssets {
    pub hex_mesh: Handle<Mesh>,
    pub materials: HashMap<u8, Handle<ColorMaterial>>,
    pub enhanced_materials: HashMap<HexCoord, Handle<ColorMaterial>>, // Store enhanced materials per tile
    pub visual_config: VisualConfig,
    pub elevation_range: (f32, f32), // min, max elevation
    pub sea_level: f32,
}

#[derive(Clone)]
pub struct VisualConfig {
    pub elevation_shading: bool,
    pub elevation_intensity: f32,
    pub water_depth_shading: bool,
    pub strategic_highlighting: bool,
    pub river_highlighting: bool,
}

impl Default for VisualConfig {
    fn default() -> Self {
        Self {
            elevation_shading: true,
            elevation_intensity: 0.3,
            water_depth_shading: true,
            strategic_highlighting: true,
            river_highlighting: true,
        }
    }
}

#[derive(Resource)]
pub struct WorldInfo {
    pub sea_level: f32,
    pub config: WorldGenConfig,
    pub total_land_tiles: usize,
    pub total_ocean_tiles: usize,
    pub biome_counts: HashMap<u8, usize>,
}

pub fn setup_map(
    mut commands: Commands, 
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    setup_map_with_config(commands, meshes, materials, WorldGenConfig::default());
}

pub fn setup_map_with_config(
    mut commands: Commands, 
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    config: WorldGenConfig,
) {
    println!("=== GENERATING REALISTIC WORLD ===");
    println!("World Type: {} continents, {:.0}% land target", 
             config.continent_count, 
             config.target_land_percentage * 100.0);
    
    let hex_mesh = create_hexagon_mesh(HEX_SIZE);
    let mesh_handle = meshes.add(hex_mesh);
    
    // Generate the world using our configurable system
    let mut world_gen = WorldGenerator::with_config(MAP_RADIUS, config.clone());
    let world_tiles = world_gen.generate();
    
    // Pre-create materials for all biome types with elevation shading
    let mut biome_materials = HashMap::new();
    let visual_config = VisualConfig::default();
    
    // Calculate elevation range for shading
    let min_elevation = world_tiles.iter().map(|t| t.elevation).fold(f32::INFINITY, f32::min);
    let max_elevation = world_tiles.iter().map(|t| t.elevation).fold(f32::NEG_INFINITY, f32::max);
    let elevation_range = max_elevation - min_elevation;
    
    println!("Elevation range: {:.3} to {:.3} (range: {:.3})", min_elevation, max_elevation, elevation_range);
    
    for biome_id in 0..=62u8 {
        let biome_type = BiomeType::from_u8(biome_id);
        let base_color = biome_type.color();
        let material_handle = materials.add(ColorMaterial::from(base_color));
        biome_materials.insert(biome_id, material_handle);
    }
    
    commands.insert_resource(TerrainAssets {
        hex_mesh: mesh_handle.clone(),
        materials: biome_materials.clone(),
        enhanced_materials: HashMap::new(),
        visual_config: visual_config.clone(),
        elevation_range: (min_elevation, max_elevation),
        sea_level: world_gen.sea_level,
    });

    // Track statistics
    let mut tiles_created = 0;
    let mut rivers_created = 0;
    let mut coastal_tiles = 0;
    let mut total_land_tiles = 0;
    let mut total_ocean_tiles = 0;
    let mut biome_counts = HashMap::new();
    
    // Create map tiles from world generation
    for world_tile in world_tiles {
        let world_pos = world_tile.hex_coord.to_world_pos(HEX_SIZE);
        let elevation_u8 = ((world_tile.elevation + 1.0) * 127.5).clamp(0.0, 255.0) as u8;
        let material_handle = biome_materials[&world_tile.biome].clone();

        // Calculate water distance (simplified)
        let water_distance = if world_tile.elevation <= world_gen.sea_level {
            0
        } else if world_tile.is_coastal {
            1
        } else {
            ((world_tile.elevation - world_gen.sea_level) * 10.0) as u8
        };

        commands.spawn((
            MapTile {
                hex_coord: world_tile.hex_coord,
                terrain: world_tile.biome, // Use biome as terrain for compatibility
                biome: world_tile.biome,
                elevation: elevation_u8,
                elevation_raw: world_tile.elevation,
                resource: world_tile.resource,
                has_river: world_tile.has_river,
                river_flow: world_tile.river_flow,
                is_coastal: world_tile.is_coastal,
                water_distance,
                temperature: world_tile.temperature,
                precipitation: world_tile.precipitation,
                soil_fertility: world_tile.soil_fertility,
                geology: world_tile.geology,
                strategic_feature: world_tile.strategic_feature,
                defensibility: world_tile.defensibility,
                trade_value: world_tile.trade_value,
                flood_risk: world_tile.flood_risk,
                naval_access: world_tile.naval_access,
            },
            Mesh2d(mesh_handle.clone()),
            MeshMaterial2d(material_handle),
            Transform::from_translation(Vec3::new(world_pos.x, world_pos.y, 0.0)),
        ));
        
        // Update statistics
        tiles_created += 1;
        if world_tile.has_river { rivers_created += 1; }
        if world_tile.is_coastal { coastal_tiles += 1; }
        
        if world_tile.elevation <= world_gen.sea_level {
            total_ocean_tiles += 1;
        } else {
            total_land_tiles += 1;
        }
        
        *biome_counts.entry(world_tile.biome).or_insert(0) += 1;
    }
    
    // Store world information for reference
    commands.insert_resource(WorldInfo {
        sea_level: world_gen.sea_level,
        config: config.clone(),
        total_land_tiles,
        total_ocean_tiles,
        biome_counts: biome_counts.clone(),
    });
    
    println!("=== WORLD GENERATION COMPLETE ===");
    println!("Created {} tiles", tiles_created);
    println!("Rivers: {}", rivers_created);
    println!("Coastal tiles: {}", coastal_tiles);
    println!("Land/Ocean ratio: {:.1}% land", 
             (total_land_tiles as f32 / tiles_created as f32) * 100.0);
    println!("Sea level: {:.3}", world_gen.sea_level);
    
    // Print biome distribution
    println!("=== BIOME DISTRIBUTION ===");
    let mut biome_list: Vec<_> = biome_counts.iter().collect();
    biome_list.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count, descending
    
    for (biome_id, count) in biome_list.iter().take(10) {
        let biome_type = BiomeType::from_u8(**biome_id);
        let percentage = (**count as f32 / tiles_created as f32) * 100.0;
        println!("{:?}: {} tiles ({:.1}%)", biome_type, count, percentage);
    }
}

fn create_hexagon_mesh(size: f32) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    
    // Center vertex
    vertices.push([0.0, 0.0, 0.0]);
    
    // Outer vertices (flat-top hexagon)
    for i in 0..6 {
        let angle = PI / 3.0 * i as f32 + PI / 6.0; // Start at 30 degrees for flat-top
        let x = size * angle.cos();
        let y = size * angle.sin();
        vertices.push([x, y, 0.0]);
    }
    
    // Create triangles from center to each edge
    for i in 0..6 {
        let next = if i == 5 { 1 } else { i + 2 };
        indices.extend_from_slice(&[0, i + 1, next]);
    }
    
    Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_indices(bevy::render::mesh::Indices::U32(indices))
}

// Helper function to get climate description
pub fn get_climate_description(temperature: f32, precipitation: f32) -> String {
    let temp_desc = match temperature {
        t if t < 0.2 => "Frigid",
        t if t < 0.4 => "Cold", 
        t if t < 0.6 => "Cool",
        t if t < 0.8 => "Warm",
        _ => "Hot",
    };
    
    let precip_desc = match precipitation {
        p if p < 0.2 => "Arid",
        p if p < 0.4 => "Dry",
        p if p < 0.6 => "Moderate",
        p if p < 0.8 => "Wet",
        _ => "Very Wet",
    };
    
    format!("{}, {}", temp_desc, precip_desc)
}

// Helper function to evaluate tile suitability for different purposes
pub fn evaluate_tile_suitability(tile: &MapTile) -> TileSuitability {
    let (food, production, _) = TerrainType::from_u8(tile.terrain).base_yields();
    
    TileSuitability {
        agriculture: tile.soil_fertility * (food / 3.0).min(1.0),
        industry: (production / 3.0).min(1.0),
        settlement: calculate_settlement_suitability(tile),
        defensibility: calculate_defensibility(tile),
    }
}

#[derive(Debug)]
pub struct TileSuitability {
    pub agriculture: f32,     // 0.0 to 1.0
    pub industry: f32,        // 0.0 to 1.0
    pub settlement: f32,      // 0.0 to 1.0
    pub defensibility: f32,   // 0.0 to 1.0
}

fn calculate_settlement_suitability(tile: &MapTile) -> f32 {
    let mut suitability: f32 = 0.5; // Base suitability
    
    // Fresh water access
    if tile.has_river { suitability += 0.3; }
    
    // Coastal access (trade)
    if tile.is_coastal { suitability += 0.2; }
    
    // Climate suitability
    let climate_bonus = match (tile.temperature, tile.precipitation) {
        (t, p) if t > 0.3 && t < 0.8 && p > 0.2 && p < 0.8 => 0.2, // Temperate
        _ => -0.1, // Extreme climates
    };
    suitability += climate_bonus;
    
    // Avoid deserts and extreme cold
    let biome = BiomeType::from_u8(tile.biome);
    match biome {
        BiomeType::HotDesert | BiomeType::ColdDesert => suitability -= 0.3,
        BiomeType::TundraBarren | BiomeType::AlpineTundra => suitability -= 0.2,
        _ => {}
    }
    
    suitability.clamp(0.0, 1.0)
}

fn calculate_defensibility(tile: &MapTile) -> f32 {
    let mut defensibility = 0.3; // Base defensibility
    
    // Elevation advantage
    defensibility += (tile.elevation_raw * 0.5).min(0.4);
    
    // River defense bonus
    if tile.has_river { defensibility += 0.2; }
    
    // Coastal tiles are harder to defend (more approaches)
    if tile.is_coastal { defensibility -= 0.1; }
    
    // Forest/jungle provides concealment
    let biome = BiomeType::from_u8(tile.biome);
    match biome {
        BiomeType::TropicalRainforest | BiomeType::TemperateDeciduousForest => {
            defensibility += 0.2;
        }
        _ => {}
    }
    
    defensibility.clamp(0.0, 1.0)
}

// === VISUAL ENHANCEMENT SYSTEM ===

fn calculate_enhanced_color(
    tile: &WorldTile,
    config: &VisualConfig,
    min_elevation: f32,
    max_elevation: f32,
    sea_level: f32,
) -> Color {
    let biome_type = BiomeType::from_u8(tile.biome);
    let mut base_color = biome_type.color();
    
    // Apply elevation-based shading
    if config.elevation_shading {
        base_color = apply_elevation_shading(base_color, tile, min_elevation, max_elevation, sea_level, config.elevation_intensity);
    }
    
    // Apply water depth shading for ocean tiles
    if config.water_depth_shading && tile.elevation <= sea_level {
        base_color = apply_water_depth_shading(base_color, tile, sea_level);
    }
    
    // Apply strategic feature highlighting
    if config.strategic_highlighting && tile.strategic_feature != 0 {
        base_color = apply_strategic_highlighting(base_color, tile);
    }
    
    // Apply river highlighting
    if config.river_highlighting && tile.has_river {
        base_color = apply_river_highlighting(base_color, tile);
    }
    
    base_color
}

fn apply_elevation_shading(
    base_color: Color,
    tile: &WorldTile,
    min_elevation: f32,
    max_elevation: f32,
    sea_level: f32,
    intensity: f32,
) -> Color {
    let elevation_range = max_elevation - min_elevation;
    if elevation_range <= 0.0 {
        return base_color;
    }
    
    // Calculate relative elevation (0.0 to 1.0)
    let relative_elevation = (tile.elevation - min_elevation) / elevation_range;
    
    // Different shading for land vs water
    let shading_factor = if tile.elevation > sea_level {
        // Land: higher = lighter, lower = darker
        let land_elevation = (tile.elevation - sea_level) / (max_elevation - sea_level);
        0.5 + (land_elevation - 0.5) * intensity
    } else {
        // Water: deeper = darker
        let water_depth = (sea_level - tile.elevation) / (sea_level - min_elevation);
        1.0 - water_depth * intensity * 0.7
    };
    
    // Apply shading to RGB channels
    let srgba = base_color.to_srgba();
    Color::srgb(
        (srgba.red * shading_factor).clamp(0.0, 1.0),
        (srgba.green * shading_factor).clamp(0.0, 1.0),
        (srgba.blue * shading_factor).clamp(0.0, 1.0),
    )
}

fn apply_water_depth_shading(base_color: Color, tile: &super::world_gen::WorldTile, sea_level: f32) -> Color {
    let depth = sea_level - tile.elevation;
    let depth_factor = (depth * 2.0).min(1.0); // Normalize depth
    
    // Deeper water = darker blue
    let srgba = base_color.to_srgba();
    Color::srgb(
        srgba.red * (1.0 - depth_factor * 0.4),
        srgba.green * (1.0 - depth_factor * 0.3),
        srgba.blue * (1.0 - depth_factor * 0.1), // Keep blue prominent
    )
}

fn apply_strategic_highlighting(base_color: Color, tile: &super::world_gen::WorldTile) -> Color {
    use super::world_gen::StrategicFeature;
    
    let feature = StrategicFeature::from_u8(tile.strategic_feature);
    let highlight_color = match feature {
        StrategicFeature::RiverDelta => Color::srgb(0.2, 0.8, 0.2), // Green for fertility
        StrategicFeature::HighlandFortress | StrategicFeature::Plateau => Color::srgb(0.8, 0.6, 0.4), // Brown for defense
        StrategicFeature::NaturalHarbor | StrategicFeature::Strait => Color::srgb(0.4, 0.6, 1.0), // Blue for naval
        StrategicFeature::DesertOasis => Color::srgb(0.2, 0.9, 0.8), // Cyan for water in desert
        StrategicFeature::MountainPass => Color::srgb(0.9, 0.8, 0.5), // Yellow for passage
        _ => base_color, // No highlight for other features
    };
    
    if highlight_color != base_color {
        // Subtle highlighting - blend base color with highlight
        let srgba = base_color.to_srgba();
        let highlight_srgba = highlight_color.to_srgba();
        Color::srgb(
            srgba.red * 0.85 + highlight_srgba.red * 0.15,
            srgba.green * 0.85 + highlight_srgba.green * 0.15,
            srgba.blue * 0.85 + highlight_srgba.blue * 0.15,
        )
    } else {
        base_color
    }
}

fn apply_river_highlighting(base_color: Color, tile: &WorldTile) -> Color {
    // Subtle blue tint for tiles with rivers
    let river_intensity = tile.river_flow * 0.2; // Scale by river flow
    let srgba = base_color.to_srgba();
    
    Color::srgb(
        srgba.red * (1.0 - river_intensity * 0.3),
        srgba.green * (1.0 - river_intensity * 0.1),
        srgba.blue.min(1.0), // Keep blue channel
    )
}

// Visual configuration toggle functions
pub fn toggle_elevation_shading(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut terrain_assets: ResMut<TerrainAssets>,
) {
    if keyboard.just_pressed(KeyCode::KeyE) {
        terrain_assets.visual_config.elevation_shading = !terrain_assets.visual_config.elevation_shading;
        println!("Elevation shading: {}", 
                if terrain_assets.visual_config.elevation_shading { "ON" } else { "OFF" });
    }
}

pub fn adjust_elevation_intensity(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut terrain_assets: ResMut<TerrainAssets>,
) {
    let mut changed = false;
    
    if keyboard.just_pressed(KeyCode::BracketLeft) {
        terrain_assets.visual_config.elevation_intensity = 
            (terrain_assets.visual_config.elevation_intensity - 0.1).max(0.0);
        changed = true;
    }
    
    if keyboard.just_pressed(KeyCode::BracketRight) {
        terrain_assets.visual_config.elevation_intensity = 
            (terrain_assets.visual_config.elevation_intensity + 0.1).min(1.0);
        changed = true;
    }
    
    if changed {
        println!("Elevation intensity: {:.1}", terrain_assets.visual_config.elevation_intensity);
    }
}

// Convenience functions for different world types
pub fn setup_pangaea_world(
    commands: Commands, 
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    setup_map_with_config(commands, meshes, materials, WorldGenConfig::pangaea());
}

pub fn setup_archipelago_world(
    commands: Commands, 
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    setup_map_with_config(commands, meshes, materials, WorldGenConfig::archipelago_world());
}

pub fn setup_fragmented_world(
    commands: Commands, 
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    setup_map_with_config(commands, meshes, materials, WorldGenConfig::fragmented_continents());
}

pub fn setup_dual_supercontinents(
    commands: Commands, 
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    setup_map_with_config(commands, meshes, materials, WorldGenConfig::dual_supercontinents());
}

pub fn setup_mediterranean_world(
    commands: Commands, 
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    setup_map_with_config(commands, meshes, materials, WorldGenConfig::mediterranean_world());
}

// Example of custom world configuration:
// pub fn setup_custom_world(
//     commands: Commands, 
//     meshes: ResMut<Assets<Mesh>>,
//     materials: ResMut<Assets<ColorMaterial>>,
// ) {
//     let custom_config = WorldGenConfig {
//         continent_count: 3,
//         continent_size: 1.5,
//         target_land_percentage: 0.4,
//         global_temperature: 0.9,
//         rainfall_multiplier: 1.2,
//         island_frequency: 1.5,
//         tectonic_activity: 1.3,
//         ..Default::default()
//     };
//     setup_map_with_config(commands, meshes, materials, custom_config);
// }