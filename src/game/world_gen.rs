use bevy::prelude::*;
use super::hex::HexCoord;
use noise::{NoiseFn, Perlin, RidgedMulti};
use std::collections::HashMap;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct WorldGenConfig {
    // Continental Configuration
    pub continent_count: usize,           // 1-8 major landmasses
    pub continent_size: f32,              // 0.5-2.0, affects influence radius
    pub continent_separation: f32,        // 0.5-2.0, spacing between continents
    pub continent_clustering: f32,        // 0.0-1.0, how grouped continents are
    
    // Ocean/Land Balance
    pub target_land_percentage: f32,     // 0.2-0.8, desired land/ocean ratio
    pub sea_level_variance: f32,         // 0.0-0.3, how much sea level can vary
    
    // Geological Activity
    pub tectonic_activity: f32,          // 0.5-2.0, mountain formation intensity
    pub volcanic_activity: f32,          // 0.0-2.0, volcanic island formation
    
    // Climate Modifiers
    pub global_temperature: f32,         // 0.3-1.0, overall world warmth
    pub rainfall_multiplier: f32,        // 0.5-1.5, global wetness
    pub climate_extremeness: f32,        // 0.5-2.0, how varied climate zones are
    
    // Special Features
    pub island_frequency: f32,           // 0.0-2.0, volcanic/isolated islands
    pub archipelago_zones: usize,        // 0-4, number of island chain regions
    pub inland_seas: bool,               // Large enclosed water bodies
}

impl Default for WorldGenConfig {
    fn default() -> Self {
        Self {
            continent_count: 4,
            continent_size: 1.0,
            continent_separation: 1.0,
            continent_clustering: 0.5,
            target_land_percentage: 0.35,
            sea_level_variance: 0.1,
            tectonic_activity: 1.0,
            volcanic_activity: 1.0,
            global_temperature: 1.0,
            rainfall_multiplier: 0.9,
            climate_extremeness: 1.0,
            island_frequency: 1.0,
            archipelago_zones: 1,
            inland_seas: false,
        }
    }
}

impl WorldGenConfig {
    // Preset world types for easy selection
    pub fn pangaea() -> Self {
        Self {
            continent_count: 1,
            continent_size: 2.5,
            continent_separation: 1.0,
            continent_clustering: 0.0,
            target_land_percentage: 0.45,
            island_frequency: 0.3,
            ..Default::default()
        }
    }
    
    pub fn archipelago_world() -> Self {
        Self {
            continent_count: 2,
            continent_size: 0.6,
            continent_separation: 2.0,
            continent_clustering: 0.2,
            target_land_percentage: 0.25,
            island_frequency: 2.5,
            archipelago_zones: 4,
            volcanic_activity: 1.8,
            ..Default::default()
        }
    }
    
    pub fn fragmented_continents() -> Self {
        Self {
            continent_count: 7,
            continent_size: 0.7,
            continent_separation: 1.5,
            continent_clustering: 0.8,
            target_land_percentage: 0.32,
            island_frequency: 1.4,
            ..Default::default()
        }
    }
    
    pub fn dual_supercontinents() -> Self {
        Self {
            continent_count: 2,
            continent_size: 1.8,
            continent_separation: 2.5,
            continent_clustering: 0.1,
            target_land_percentage: 0.40,
            inland_seas: true,
            ..Default::default()
        }
    }
    
    pub fn mediterranean_world() -> Self {
        Self {
            continent_count: 4,
            continent_size: 1.2,
            continent_separation: 0.8,
            continent_clustering: 0.9,
            target_land_percentage: 0.42,
            inland_seas: true,
            tectonic_activity: 1.3,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorldTile {
    pub hex_coord: HexCoord,
    pub elevation: f32,          // Raw elevation value (-1.0 to 1.0)
    pub terrain: u8,
    pub biome: u8,               // New: separate biome from terrain
    pub has_river: bool,
    pub river_flow: f32,         // New: river flow strength (0.0 to 1.0)
    pub river_edges: [bool; 6],  // Which edges have rivers
    pub is_coastal: bool,
    pub resource: u8,
    pub temperature: f32,        // Annual average temperature (-1.0 to 1.0)
    pub precipitation: f32,      // Annual precipitation (0.0 to 1.0)
    pub drainage: f32,           // How well water drains (affects wetlands)
    pub geology: u8,             // Geological formation type
    pub soil_fertility: f32,     // Agricultural potential
    
    // Strategic Geography Features
    pub strategic_feature: u8,   // Type of strategic feature (0 = none)
    pub defensibility: f32,      // How defensible this position is (0.0 to 1.0)
    pub trade_value: f32,        // Economic/trade importance (0.0 to 1.0)
    pub flood_risk: f32,         // Risk of flooding (0.0 to 1.0)
    pub naval_access: f32,       // Naval movement/access value (0.0 to 1.0)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BiomeType {
    // Aquatic
    Ocean = 0,
    Lake = 1,
    River = 2,
    
    // Cold
    TundraBarren = 10,
    TundraWet = 11,
    TaigaBorealForest = 12,
    
    // Temperate
    TemperateGrassland = 20,
    TemperateDeciduousForest = 21,
    TemperateConiferForest = 22,
    TemperateRainforest = 23,
    
    // Warm/Hot
    TropicalGrasslandSavanna = 30,
    TropicalSeasonalForest = 31,
    TropicalRainforest = 32,
    
    // Dry
    ColdDesert = 40,
    HotDesert = 41,
    Shrubland = 42,
    
    // High altitude
    AlpineTundra = 50,
    MontaneForest = 51,
    
    // Coastal/Wetland
    Mangrove = 60,
    SaltMarsh = 61,
    Wetland = 62,
}

impl BiomeType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => BiomeType::Ocean,
            1 => BiomeType::Lake,
            2 => BiomeType::River,
            10 => BiomeType::TundraBarren,
            11 => BiomeType::TundraWet,
            12 => BiomeType::TaigaBorealForest,
            20 => BiomeType::TemperateGrassland,
            21 => BiomeType::TemperateDeciduousForest,
            22 => BiomeType::TemperateConiferForest,
            23 => BiomeType::TemperateRainforest,
            30 => BiomeType::TropicalGrasslandSavanna,
            31 => BiomeType::TropicalSeasonalForest,
            32 => BiomeType::TropicalRainforest,
            40 => BiomeType::ColdDesert,
            41 => BiomeType::HotDesert,
            42 => BiomeType::Shrubland,
            50 => BiomeType::AlpineTundra,
            51 => BiomeType::MontaneForest,
            60 => BiomeType::Mangrove,
            61 => BiomeType::SaltMarsh,
            62 => BiomeType::Wetland,
            _ => BiomeType::TemperateGrassland,
        }
    }

    pub fn color(self) -> Color {
        match self {
            BiomeType::Ocean => Color::srgb(0.1, 0.3, 0.8),
            BiomeType::Lake => Color::srgb(0.2, 0.5, 0.9),
            BiomeType::River => Color::srgb(0.3, 0.6, 1.0),
            
            BiomeType::TundraBarren => Color::srgb(0.8, 0.9, 1.0),
            BiomeType::TundraWet => Color::srgb(0.7, 0.8, 0.9),
            BiomeType::TaigaBorealForest => Color::srgb(0.2, 0.5, 0.3),
            
            BiomeType::TemperateGrassland => Color::srgb(0.5, 0.8, 0.3),
            BiomeType::TemperateDeciduousForest => Color::srgb(0.2, 0.7, 0.2),
            BiomeType::TemperateConiferForest => Color::srgb(0.1, 0.6, 0.2),
            BiomeType::TemperateRainforest => Color::srgb(0.0, 0.5, 0.1),
            
            BiomeType::TropicalGrasslandSavanna => Color::srgb(0.8, 0.7, 0.3),
            BiomeType::TropicalSeasonalForest => Color::srgb(0.4, 0.8, 0.3),
            BiomeType::TropicalRainforest => Color::srgb(0.0, 0.4, 0.0),
            
            BiomeType::ColdDesert => Color::srgb(0.7, 0.7, 0.8),
            BiomeType::HotDesert => Color::srgb(0.9, 0.8, 0.4),
            BiomeType::Shrubland => Color::srgb(0.6, 0.6, 0.4),
            
            BiomeType::AlpineTundra => Color::srgb(0.9, 0.9, 1.0),
            BiomeType::MontaneForest => Color::srgb(0.3, 0.6, 0.4),
            
            BiomeType::Mangrove => Color::srgb(0.3, 0.5, 0.3),
            BiomeType::SaltMarsh => Color::srgb(0.5, 0.6, 0.4),
            BiomeType::Wetland => Color::srgb(0.4, 0.7, 0.5),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrategicFeature {
    None = 0,
    RiverDelta = 1,           // Where major rivers meet the ocean
    Peninsula = 2,            // Land extending into water
    Cape = 3,                 // Narrow coastal protrusion
    Strait = 4,               // Narrow water passage
    NaturalHarbor = 5,        // Protected coastal inlet
    MountainPass = 6,         // Route through mountains
    Canyon = 7,               // Deep river valley
    IslandChain = 8,          // Connected island group
    Plateau = 9,              // Elevated defensive position
    Isthmus = 10,             // Narrow land bridge
    Bay = 11,                 // Large coastal indentation
    Fjord = 12,               // Deep coastal valley
    DesertOasis = 13,         // Water source in arid regions
    RiverFord = 14,           // Shallow river crossing
    HighlandFortress = 15,    // Naturally defensible highland
}

impl StrategicFeature {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => StrategicFeature::RiverDelta,
            2 => StrategicFeature::Peninsula,
            3 => StrategicFeature::Cape,
            4 => StrategicFeature::Strait,
            5 => StrategicFeature::NaturalHarbor,
            6 => StrategicFeature::MountainPass,
            7 => StrategicFeature::Canyon,
            8 => StrategicFeature::IslandChain,
            9 => StrategicFeature::Plateau,
            10 => StrategicFeature::Isthmus,
            11 => StrategicFeature::Bay,
            12 => StrategicFeature::Fjord,
            13 => StrategicFeature::DesertOasis,
            14 => StrategicFeature::RiverFord,
            15 => StrategicFeature::HighlandFortress,
            _ => StrategicFeature::None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            StrategicFeature::None => "",
            StrategicFeature::RiverDelta => "River Delta",
            StrategicFeature::Peninsula => "Peninsula",
            StrategicFeature::Cape => "Cape",
            StrategicFeature::Strait => "Strait", 
            StrategicFeature::NaturalHarbor => "Natural Harbor",
            StrategicFeature::MountainPass => "Mountain Pass",
            StrategicFeature::Canyon => "Canyon",
            StrategicFeature::IslandChain => "Island Chain",
            StrategicFeature::Plateau => "Plateau",
            StrategicFeature::Isthmus => "Isthmus",
            StrategicFeature::Bay => "Bay",
            StrategicFeature::Fjord => "Fjord",
            StrategicFeature::DesertOasis => "Desert Oasis",
            StrategicFeature::RiverFord => "River Ford",
            StrategicFeature::HighlandFortress => "Highland Fortress",
        }
    }
}
pub enum GeologyType {
    OceanicCrust = 0,
    ContinentalShelf = 1,
    Sedimentary = 2,
    Igneous = 3,
    Metamorphic = 4,
    Volcanic = 5,
    Limestone = 6,
    Sandstone = 7,
    Granite = 8,
    Basalt = 9,
}

pub struct WorldGenerator {
    pub map_radius: i32,
    pub tiles: HashMap<HexCoord, WorldTile>,
    pub sea_level: f32,
    pub config: WorldGenConfig,
    pub flow_directions: HashMap<HexCoord, (usize, HexCoord)>, // (direction, target)
    pub flow_accumulation: HashMap<HexCoord, f32>, // accumulated water flow
}

impl WorldGenerator {
    pub fn new(map_radius: i32) -> Self {
        Self::with_config(map_radius, WorldGenConfig::default())
    }
    
    pub fn with_config(map_radius: i32, config: WorldGenConfig) -> Self {
        Self {
            map_radius,
            tiles: HashMap::new(),
            sea_level: 0.0,  // Will be calculated based on elevation distribution
            config,
            flow_directions: HashMap::new(),
            flow_accumulation: HashMap::new(),
        }
    }

    pub fn generate(&mut self) -> Vec<WorldTile> {
        println!("=== REALISTIC WORLD GENERATION ===");
        
        // Phase 1: Geological Foundation
        println!("Phase 1: Tectonic and geological formation...");
        self.generate_tectonic_structure();
        self.generate_base_elevation();
        self.apply_geological_processes();
        self.determine_sea_level();
        
        // Phase 2: Hydrological Cycle
        println!("Phase 2: Hydrological systems...");
        self.create_drainage_basins();
        self.generate_rivers();
        self.mark_coastal_features();
        
        // Phase 3: Climate Simulation
        println!("Phase 3: Climate systems...");
        self.simulate_temperature();
        self.simulate_precipitation();
        self.apply_orographic_effects(); // Rain shadows
        
        // Phase 4: Ecological Systems
        println!("Phase 4: Biome assignment...");
        self.assign_biomes();
        self.refine_river_network(); // Add more rivers in appropriate biomes
        self.place_lakes(); // After biomes are assigned for better threshold calculation
        self.calculate_soil_fertility();
        
        // Debug climate ranges
        let temps: Vec<f32> = self.tiles.values().map(|t| t.temperature).collect();
        let precips: Vec<f32> = self.tiles.values().map(|t| t.precipitation).collect();
        let min_temp = temps.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_temp = temps.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let min_precip = precips.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_precip = precips.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        println!("Temperature range: {:.3} to {:.3}", min_temp, max_temp);
        println!("Precipitation range: {:.3} to {:.3}", min_precip, max_precip);
        
        // Phase 5: Resource Distribution
        println!("Phase 5: Resource placement...");
        self.place_geological_resources();
        self.place_biological_resources();
        
        println!("World generation complete! {} tiles created", self.tiles.len());
        
        self.tiles.values().cloned().collect()
    }

    fn generate_tectonic_structure(&mut self) {
        // Create the basic tectonic structure with continental and oceanic plates
        let mut rng = rand::rng();
        let plate_noise = RidgedMulti::<Perlin>::new(rng.random());
        
        // Generate continental centers based on configuration
        let continent_centers = self.generate_continent_centers();
        
        println!("Generated {} continental centers", continent_centers.len());
        
        for q in -self.map_radius..=self.map_radius {
            let r1 = (-self.map_radius).max(-q - self.map_radius);
            let r2 = (self.map_radius).min(-q + self.map_radius);
            for r in r1..=r2 {
                let hex_coord = HexCoord::new(q, r);
                
                // Distance to nearest continental center with size scaling
                let min_continent_distance = continent_centers.iter()
                    .map(|&center| self.hex_distance(hex_coord, center) as f32)
                    .fold(f32::INFINITY, f32::min);
                
                // Continental influence decreases with distance, affected by continent size
                let influence_radius = 40.0 * self.config.continent_size;
                let continent_influence = (-min_continent_distance / influence_radius).exp();
                
                // Plate boundaries (creates mountain ranges and rift valleys)
                let plate_scale = (0.02 * self.config.tectonic_activity) as f64;
                let plate_value = plate_noise.get([
                    hex_coord.q as f64 * plate_scale,
                    hex_coord.r as f64 * plate_scale,
                ]) as f32;
                
                // Base continental/oceanic determination
                let continental_base = continent_influence * 0.7 + plate_value * 0.3;
                
                // Volcanic island formation
                let volcanic_threshold = 0.8 * (2.0 - self.config.volcanic_activity);
                let volcanic_influence = if plate_value > volcanic_threshold && continental_base < 0.2 {
                    0.4 * self.config.volcanic_activity // Create volcanic islands
                } else {
                    0.0
                };
                
                let final_continental_value = continental_base + volcanic_influence;
                
                let geology = if final_continental_value > 0.3 {
                    if plate_value > 0.6 { GeologyType::Granite } // Continental core
                    else if plate_value > 0.3 { GeologyType::Metamorphic } // Mountain building
                    else { GeologyType::Sedimentary } // Stable platform
                } else if final_continental_value > 0.1 {
                    GeologyType::ContinentalShelf // Shallow seas
                } else if volcanic_influence > 0.0 {
                    GeologyType::Volcanic // Volcanic islands
                } else {
                    GeologyType::OceanicCrust // Deep ocean
                };

                let tile = WorldTile {
                    hex_coord,
                    elevation: final_continental_value, // Temporary, will be refined
                    terrain: 0,
                    biome: 0,
                    has_river: false,
                    river_flow: 0.0,
                    river_edges: [false; 6],
                    is_coastal: false,
                    resource: 0,
                    temperature: 0.0,
                    precipitation: 0.0,
                    drainage: 0.5,
                    geology: geology as u8,
                    soil_fertility: 0.0,
                    strategic_feature: 0,
                    defensibility: 0.0,
                    trade_value: 0.0,
                    flood_risk: 0.0,
                    naval_access: 0.0,
                };
                
                self.tiles.insert(hex_coord, tile);
            }
        }
    }
    
    fn generate_continent_centers(&self) -> Vec<HexCoord> {
        let mut rng = rand::rng();
        let mut centers = Vec::new();
        
        // Determine spacing based on separation config
        let base_spacing = (self.map_radius as f32 * 0.6 * self.config.continent_separation) as i32;
        let cluster_factor = self.config.continent_clustering;
        
        match self.config.continent_count {
            1 => {
                // Single supercontinent (Pangaea-style)
                centers.push(HexCoord::new(0, 0));
            },
            2 => {
                // Dual continents
                let separation = base_spacing.max(20);
                centers.push(HexCoord::new(-separation/2, 0));
                centers.push(HexCoord::new(separation/2, 0));
            },
            count => {
                // Multiple continents - distribute across latitudes and longitudes
                for i in 0..count {
                    let angle = (i as f32 / count as f32) * 2.0 * std::f32::consts::PI;
                    
                    // Base position in circle
                    let base_radius = (self.map_radius as f32 * 0.4 * self.config.continent_separation).max(20.0);
                    let mut q = (base_radius * angle.cos()) as i32;
                    let mut r = (base_radius * angle.sin()) as i32;
                    
                    // Add clustering/dispersion
                    if cluster_factor > 0.5 {
                        // Cluster continents together
                        let cluster_offset = (cluster_factor - 0.5) * 2.0;
                        q = (q as f32 * (1.0 - cluster_offset * 0.5)) as i32;
                        r = (r as f32 * (1.0 - cluster_offset * 0.5)) as i32;
                    } else {
                        // Spread continents apart
                        let spread_factor = (0.5 - cluster_factor) * 2.0;
                        q = (q as f32 * (1.0 + spread_factor)) as i32;
                        r = (r as f32 * (1.0 + spread_factor)) as i32;
                    }
                    
                    // Add some randomness
                    q += rng.random_range(-10..=10);
                    r += rng.random_range(-10..=10);
                    
                    // Ensure within bounds
                    q = q.clamp(-self.map_radius + 20, self.map_radius - 20);
                    r = r.clamp(-self.map_radius + 20, self.map_radius - 20);
                    
                    centers.push(HexCoord::new(q, r));
                }
            }
        }
        
        // Add archipelago zones if configured
        for _i in 0..self.config.archipelago_zones {
            let angle = rng.random::<f32>() * 2.0 * std::f32::consts::PI;
            let radius = self.map_radius as f32 * 0.7;
            let q = (radius * angle.cos()) as i32;
            let r = (radius * angle.sin()) as i32;
            
            // Add multiple small island centers around this point
            for j in 0..3 {
                let sub_angle = (j as f32 / 3.0) * 2.0 * std::f32::consts::PI;
                let sub_radius = 15.0;
                let sub_q = q + (sub_radius * sub_angle.cos()) as i32;
                let sub_r = r + (sub_radius * sub_angle.sin()) as i32;
                
                if sub_q.abs() < self.map_radius - 10 && sub_r.abs() < self.map_radius - 10 {
                    centers.push(HexCoord::new(sub_q, sub_r));
                }
            }
        }
        
        centers
    }

    fn generate_base_elevation(&mut self) {
        let mut rng = rand::rng();
        let mountain_noise = RidgedMulti::<Perlin>::new(rng.random());
        let hill_noise = Perlin::new(rng.random());
        let detail_noise = Perlin::new(rng.random());
        
        for tile in self.tiles.values_mut() {
            let coord = tile.hex_coord;
            let geology = GeologyType::from_u8(tile.geology);
            
            // Different elevation characteristics based on geology
            let base_elevation = match geology {
                GeologyType::OceanicCrust => -0.6,          // Deep ocean floor
                GeologyType::ContinentalShelf => -0.2,     // Shallow seas
                GeologyType::Sedimentary => 0.1,           // Low plains
                GeologyType::Limestone => 0.15,            // Slightly elevated
                GeologyType::Sandstone => 0.2,             // Plateaus
                GeologyType::Igneous | GeologyType::Granite => 0.4, // Highlands
                GeologyType::Metamorphic => 0.6,           // Mountain cores
                GeologyType::Volcanic => 0.7,              // Volcanic peaks
                GeologyType::Basalt => 0.3,                // Volcanic plains
            };
            
            // Add noise layers based on geology
            let mut elevation = base_elevation;
            
            // Mountain building (more pronounced in metamorphic/igneous areas)
            if matches!(geology, GeologyType::Metamorphic | GeologyType::Igneous | GeologyType::Granite) {
                let mountain_scale = 0.03;
                let mountain_value = mountain_noise.get([
                    coord.q as f64 * mountain_scale,
                    coord.r as f64 * mountain_scale,
                ]) as f32;
                elevation += mountain_value * 0.4;
            }
            
            // Hills and local variation
            let hill_scale = 0.08;
            let hill_value = hill_noise.get([
                coord.q as f64 * hill_scale,
                coord.r as f64 * hill_scale,
            ]) as f32;
            elevation += hill_value * 0.2;
            
            // Fine detail
            let detail_scale = 0.2;
            let detail_value = detail_noise.get([
                coord.q as f64 * detail_scale,
                coord.r as f64 * detail_scale,
            ]) as f32;
            elevation += detail_value * 0.1;
            
            // Clamp to reasonable range
            tile.elevation = elevation.clamp(-1.0, 1.0);
        }
    }

    fn apply_geological_processes(&mut self) {
        // Simulate erosion: high areas lose elevation, low areas gain sediment
        let coords: Vec<HexCoord> = self.tiles.keys().cloned().collect();
        let mut erosion_map = HashMap::new();
        
        for coord in &coords {
            let tile = &self.tiles[coord];
            if tile.elevation > 0.0 { // Only erode land
                let neighbors = coord.neighbors();
                let mut avg_neighbor_elevation = 0.0;
                let mut neighbor_count = 0;
                
                for neighbor in neighbors {
                    if let Some(neighbor_tile) = self.tiles.get(&neighbor) {
                        avg_neighbor_elevation += neighbor_tile.elevation;
                        neighbor_count += 1;
                    }
                }
                
                if neighbor_count > 0 {
                    avg_neighbor_elevation /= neighbor_count as f32;
                    let slope = tile.elevation - avg_neighbor_elevation;
                    let erosion = slope * 0.02; // Erosion proportional to slope
                    erosion_map.insert(*coord, -erosion.max(0.0));
                }
            }
        }
        
        // Apply erosion
        for (coord, erosion) in erosion_map {
            if let Some(tile) = self.tiles.get_mut(&coord) {
                tile.elevation = (tile.elevation + erosion).max(-1.0);
            }
        }
    }

    fn determine_sea_level(&mut self) {
        // Calculate sea level based on target land percentage from config
        let mut elevations: Vec<f32> = self.tiles.values().map(|t| t.elevation).collect();
        elevations.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        // Use target land percentage to set sea level
        let ocean_percentile = 1.0 - self.config.target_land_percentage;
        let mut base_index = (elevations.len() as f32 * ocean_percentile) as usize;
        base_index = base_index.min(elevations.len() - 1);
        
        let base_sea_level = elevations[base_index];
        
        // Apply variance if configured
        let variance = self.config.sea_level_variance;
        if variance > 0.0 {
            let mut rng = rand::rng();
            let adjustment = rng.random_range(-variance..variance);
            self.sea_level = base_sea_level + adjustment;
        } else {
            self.sea_level = base_sea_level;
        }
        
        // Calculate actual land percentage
        let actual_land_tiles = self.tiles.values().filter(|t| t.elevation > self.sea_level).count();
        let actual_land_percentage = actual_land_tiles as f32 / self.tiles.len() as f32;
        
        println!("Sea level set to: {:.3} (target: {:.1}% land, actual: {:.1}% land)", 
                 self.sea_level, 
                 self.config.target_land_percentage * 100.0,
                 actual_land_percentage * 100.0);
    }

    fn create_drainage_basins(&mut self) {
        // Calculate drainage for each tile based on slope and geology
        let coords: Vec<HexCoord> = self.tiles.keys().cloned().collect();
        
        for coord in coords {
            let tile = &self.tiles[&coord];
            let geology = GeologyType::from_u8(tile.geology);
            
            // Base drainage depends on geology
            let base_drainage = match geology {
                GeologyType::Limestone => 0.9,        // Highly permeable
                GeologyType::Sandstone => 0.7,        // Good drainage
                GeologyType::Sedimentary => 0.5,      // Moderate drainage
                GeologyType::Igneous | GeologyType::Granite => 0.3, // Poor drainage
                GeologyType::Metamorphic => 0.4,      // Poor to moderate
                GeologyType::Volcanic => 0.8,         // Volcanic soils drain well
                GeologyType::Basalt => 0.6,           // Moderate drainage
                _ => 0.5,
            };
            
            // Modify by slope (steeper = better drainage)
            let neighbors = coord.neighbors();
            let mut total_slope = 0.0;
            let mut neighbor_count = 0;
            
            for neighbor in neighbors {
                if let Some(neighbor_tile) = self.tiles.get(&neighbor) {
                    let slope = (tile.elevation - neighbor_tile.elevation).abs();
                    total_slope += slope;
                    neighbor_count += 1;
                }
            }
            
            let avg_slope = if neighbor_count > 0 { total_slope / neighbor_count as f32 } else { 0.0 };
            let slope_drainage_bonus = (avg_slope * 2.0).min(0.3);
            
            self.tiles.get_mut(&coord).unwrap().drainage = (base_drainage + slope_drainage_bonus).min(1.0);
        }
    }

    fn generate_rivers(&mut self) {
        println!("=== HYDROLOGICAL SIMULATION ===");
        
        // Step 1: Calculate flow direction for every tile
        self.calculate_flow_directions();
        
        // Step 2: Calculate flow accumulation (watershed collection)
        self.calculate_flow_accumulation();
        
        // Step 3: Generate river network based on flow accumulation
        self.generate_river_network();
        
        // Step 4: Calculate river flow rates
        self.calculate_river_flow_rates();
        
        let river_count = self.tiles.values().filter(|t| t.has_river).count();
        let total_flow: f32 = self.tiles.values().map(|t| t.river_flow).sum();
        println!("Created rivers on {} tiles (total flow: {:.0})", river_count, total_flow);
    }

    fn calculate_flow_directions(&mut self) {
        // For each tile, determine which neighbor it flows to
        let coords: Vec<HexCoord> = self.tiles.keys().cloned().collect();
        let mut flow_directions = HashMap::new();
        
        for coord in coords {
            let tile = &self.tiles[&coord];
            
            // Skip ocean tiles
            if tile.elevation <= self.sea_level {
                continue;
            }
            
            let neighbors = coord.neighbors();
            let mut lowest_neighbor = None;
            let mut lowest_elevation = tile.elevation;
            
            // Find the steepest downhill neighbor
            for (i, neighbor) in neighbors.iter().enumerate() {
                if let Some(neighbor_tile) = self.tiles.get(neighbor) {
                    if neighbor_tile.elevation < lowest_elevation {
                        lowest_elevation = neighbor_tile.elevation;
                        lowest_neighbor = Some((i, *neighbor));
                    }
                }
            }
            
            // Store flow direction (which hex direction to flow towards)
            if let Some((direction, target)) = lowest_neighbor {
                flow_directions.insert(coord, (direction, target));
            }
        }
        
        // Store flow directions for later use
        self.flow_directions = flow_directions;
        println!("Calculated flow directions for {} land tiles", self.flow_directions.len());
    }

    fn calculate_flow_accumulation(&mut self) {
        // Calculate how much water flows through each tile
        let coords: Vec<HexCoord> = self.tiles.keys().cloned().collect();
        let mut flow_accumulation = HashMap::new();
        
        // Initialize: every land tile contributes 1 unit of water
        for coord in &coords {
            let tile = &self.tiles[coord];
            if tile.elevation > self.sea_level {
                flow_accumulation.insert(*coord, 1.0f32);
            }
        }
        
        // Sort tiles by elevation (highest first) for proper flow calculation
        let mut sorted_coords = coords.clone();
        sorted_coords.retain(|coord| self.tiles[coord].elevation > self.sea_level);
        sorted_coords.sort_by(|a, b| {
            self.tiles[b].elevation.partial_cmp(&self.tiles[a].elevation).unwrap()
        });
        
        // Flow water from high to low elevation
        for coord in sorted_coords {
            if let Some((_, target)) = self.flow_directions.get(&coord) {
                let source_flow = flow_accumulation.get(&coord).copied().unwrap_or(0.0);
                
                // Add this tile's flow to the target tile
                *flow_accumulation.entry(*target).or_insert(0.0) += source_flow;
                
                // Add precipitation bonus based on climate
                let tile = &self.tiles[&coord];
                let precip_bonus = tile.precipitation * 0.5;
                *flow_accumulation.entry(*target).or_insert(0.0) += precip_bonus;
            }
        }
        
        self.flow_accumulation = flow_accumulation;
        
        let max_flow = self.flow_accumulation.values().fold(0.0f32, |a, &b| a.max(b));
        let tiles_with_flow = self.flow_accumulation.len();
        println!("Calculated flow accumulation: {} tiles, max flow: {:.1}", tiles_with_flow, max_flow);
    }

    fn generate_river_network(&mut self) {
        // Create rivers where flow accumulation exceeds threshold
        let mut river_tiles = 0;
        
        for (coord, &flow) in &self.flow_accumulation {
            let tile = self.tiles.get_mut(coord).unwrap();
            
            // Base threshold adjusted by precipitation and elevation
            let precip_factor = (1.0 - tile.precipitation * 0.5).max(0.3); // Wetter = lower threshold
            let elevation_factor = if tile.elevation > self.sea_level + 0.3 { 0.8 } else { 1.0 }; // Mountains = lower threshold
            
            let base_threshold = 4.0; // Base flow accumulation needed for a river
            let threshold = base_threshold * precip_factor * elevation_factor;
            
            if flow >= threshold {
                tile.has_river = true;
                river_tiles += 1;
            }
        }
        
        println!("Generated {} river tiles based on flow accumulation", river_tiles);
    }

    fn calculate_river_flow_rates(&mut self) {
        // Set river flow based on accumulated water
        let max_flow = self.flow_accumulation.values().fold(0.0f32, |a, &b| a.max(b));
        
        for (coord, &flow) in &self.flow_accumulation {
            if let Some(tile) = self.tiles.get_mut(coord) {
                if tile.has_river {
                    // Normalize flow to 0.1-1.0 range
                    tile.river_flow = (flow / max_flow).max(0.1).min(1.0);
                }
            }
        }
        
        // Set river edges based on flow directions
        self.set_river_edges();
    }

    fn set_river_edges(&mut self) {
        // Connect rivers along flow directions
        let flow_directions = self.flow_directions.clone();
        
        for (source_coord, (direction, target_coord)) in flow_directions {
            let source_has_river = self.tiles.get(&source_coord)
                .map(|t| t.has_river).unwrap_or(false);
            let target_has_river = self.tiles.get(&target_coord)
                .map(|t| t.has_river).unwrap_or(false);
            
            // Draw river edge if either source or target has a river
            if source_has_river || target_has_river {
                if let Some(source_tile) = self.tiles.get_mut(&source_coord) {
                    source_tile.river_edges[direction] = true;
                }
                if let Some(target_tile) = self.tiles.get_mut(&target_coord) {
                    target_tile.river_edges[(direction + 3) % 6] = true;
                }
            }
        }
    }

    fn mark_coastal_features(&mut self) {
        let coords: Vec<HexCoord> = self.tiles.keys().cloned().collect();
        
        for coord in coords {
            let tile = &self.tiles[&coord];
            
            if tile.elevation > self.sea_level {
                // Check if adjacent to ocean
                let is_coastal = coord.neighbors().iter().any(|&neighbor| {
                    self.tiles.get(&neighbor)
                        .map(|t| t.elevation <= self.sea_level)
                        .unwrap_or(false)
                });
                
                self.tiles.get_mut(&coord).unwrap().is_coastal = is_coastal;
            }
        }
    }

    fn simulate_temperature(&mut self) {
        let mut rng = rand::rng();
        let temp_noise = Perlin::new(rng.random());
        
        let coords: Vec<HexCoord> = self.tiles.keys().cloned().collect();
        
        for coord in coords {
            let ocean_distance = self.distance_to_ocean(coord);
            
            // Base temperature from latitude (distance from equator)
            let latitude_factor = (coord.r as f32 * 0.004).abs(); // Even gentler gradient for large tropical zones
            let base_temp = (1.0 - latitude_factor * 0.8).max(0.2); // Ensure minimum warmth, larger tropical zone
            
            let tile = &self.tiles[&coord];
            
            // Elevation cooling (lapse rate: ~6.5°C per 1000m)
            let elevation_cooling = if tile.elevation > self.sea_level {
                (tile.elevation - self.sea_level) * 1.5
            } else { 0.0 };
            
            // Ocean moderation (water heats/cools more slowly)
            let continental_effect = (ocean_distance / 20.0).min(0.3); // Continental climates are more extreme
            
            // Random variation
            let temp_variation = temp_noise.get([
                coord.q as f64 * 0.05,
                coord.r as f64 * 0.05,
            ]) as f32 * 0.1;
            
            let temperature = (base_temp - elevation_cooling + temp_variation + continental_effect * 0.1)
                .clamp(0.0, 1.0);
            
            // Update the tile
            self.tiles.get_mut(&coord).unwrap().temperature = temperature * self.config.global_temperature;
        }
    }

    fn simulate_precipitation(&mut self) {
        let mut rng = rand::rng();
        let precip_noise = Perlin::new(rng.random());
        
        let coords: Vec<HexCoord> = self.tiles.keys().cloned().collect();
        
        for coord in coords {
            let ocean_distance = self.distance_to_ocean(coord);
            
            // Base precipitation from latitude with more variation
            let latitude = (coord.r as f32 * 0.004).abs(); // Match temperature scaling
            let latitude_precip = if latitude < 0.15 {
                0.8 - latitude * 0.3  // Tropical high precipitation (but some variation)
            } else if latitude < 0.3 {
                0.3 + (latitude - 0.15) * 0.8  // Subtropical - can be dry (deserts)
            } else if latitude < 0.5 {
                0.5 + (latitude - 0.3) * 0.6  // Temperate - moderate precipitation
            } else {
                0.4 - (latitude - 0.5) * 0.6  // Polar - low precipitation
            }.max(0.1).min(0.9);
            
            // Ocean proximity (coastal areas get more rain, but not overwhelmingly)
            let coastal_bonus = (1.0 - (ocean_distance / 20.0).min(1.0)) * 0.2;
            
            // Random variation (larger for more diversity)
            let base_variation = 0.4 * self.config.climate_extremeness;
            let precip_variation = precip_noise.get([
                coord.q as f64 * 0.04,
                coord.r as f64 * 0.04,
            ]) as f32 * base_variation; // Increased variation
            
            let tile = &self.tiles[&coord];
            
            // Elevation effect (mountains can increase or decrease precipitation)
            let elevation_effect = if tile.elevation > self.sea_level + 0.3 {
                0.2  // Mountains force orographic precipitation
            } else { 0.0 };
            
            let precipitation = (latitude_precip + coastal_bonus + elevation_effect + precip_variation)
                .clamp(0.0, 1.0) * self.config.rainfall_multiplier;
            
            // Update the tile
            self.tiles.get_mut(&coord).unwrap().precipitation = precipitation;
        }
    }

    fn apply_orographic_effects(&mut self) {
        // Create rain shadows behind mountain ranges
        let coords: Vec<HexCoord> = self.tiles.keys().cloned().collect();
        let mut rain_shadow_effects = HashMap::new();
        
        for coord in coords {
            let tile = &self.tiles[&coord];
            
            // If this is a high elevation tile, check for rain shadows
            if tile.elevation > self.sea_level + 0.3 {
                // Check in each direction for lower elevation tiles (leeward side)
                for direction in 0..6 {
                    let mut shadow_coord = coord;
                    let mut shadow_strength = 0.3; // Starting rain shadow strength
                    
                    // Cast shadow for several hexes
                    for _distance in 1..8 {
                        shadow_coord = self.step_in_direction(shadow_coord, direction);
                        
                        if let Some(shadow_tile) = self.tiles.get(&shadow_coord) {
                            if shadow_tile.elevation < tile.elevation - 0.1 {
                                // This tile is in the rain shadow
                                let current_effect: f32 = rain_shadow_effects.get(&shadow_coord).copied().unwrap_or(0.0);
                                rain_shadow_effects.insert(shadow_coord, current_effect.max(shadow_strength));
                                shadow_strength *= 0.7; // Diminish with distance
                            } else {
                                break; // Hit another mountain
                            }
                        } else {
                            break; // Off map
                        }
                    }
                }
            }
        }
        
        // Apply rain shadow effects
        for (coord, reduction) in rain_shadow_effects {
            if let Some(tile) = self.tiles.get_mut(&coord) {
                tile.precipitation = (tile.precipitation * (1.0 - reduction)).max(0.0);
            }
        }
    }

    fn assign_biomes(&mut self) {
        let coords: Vec<HexCoord> = self.tiles.keys().cloned().collect();
        
        for coord in coords {
            let tile = &self.tiles[&coord];
            // Skip if already assigned (like lakes)
            if tile.biome != 0 { continue; }
            
            let biome = if tile.elevation <= self.sea_level {
                BiomeType::Ocean
            } else {
                self.determine_terrestrial_biome(tile)
            };
            
            // Update the tile
            self.tiles.get_mut(&coord).unwrap().biome = biome as u8;
            // Set terrain to match biome for compatibility
            self.tiles.get_mut(&coord).unwrap().terrain = biome as u8;
        }
    }

    fn determine_terrestrial_biome(&self, tile: &WorldTile) -> BiomeType {
        let temp = tile.temperature;
        let precip = tile.precipitation;
        let elevation_above_sea = tile.elevation - self.sea_level;
        
        // High altitude biomes
        if elevation_above_sea > 0.6 {
            return BiomeType::AlpineTundra;
        } else if elevation_above_sea > 0.4 && temp < 0.6 {
            return BiomeType::MontaneForest;
        }
        
        // Wetland check (poor drainage + high precipitation)
        if tile.drainage < 0.3 && precip > 0.6 && tile.is_coastal {
            return if temp > 0.7 { BiomeType::Mangrove } else { BiomeType::SaltMarsh };
        } else if tile.drainage < 0.4 && precip > 0.7 {
            return BiomeType::Wetland;
        }
        
        // Main biome classification using Whittaker diagram principles
        match temp {
            t if t < 0.2 => {
                // Very cold climates (polar)
                if precip > 0.4 { BiomeType::TundraWet } else { BiomeType::TundraBarren }
            },
            t if t < 0.4 => {
                // Cold climates (boreal)
                match precip {
                    p if p > 0.5 => BiomeType::TaigaBorealForest,
                    p if p > 0.2 => BiomeType::TaigaBorealForest,
                    _ => BiomeType::ColdDesert,
                }
            },
            t if t < 0.6 => {
                // Cool temperate climates
                match precip {
                    p if p > 0.7 => BiomeType::TemperateRainforest,
                    p if p > 0.5 => BiomeType::TemperateDeciduousForest,
                    p if p > 0.3 => BiomeType::TemperateConiferForest,
                    p if p > 0.15 => BiomeType::TemperateGrassland,
                    _ => BiomeType::ColdDesert,
                }
            },
            t if t < 0.8 => {
                // Warm temperate/subtropical climates
                match precip {
                    p if p > 0.7 => BiomeType::TropicalSeasonalForest,
                    p if p > 0.5 => BiomeType::TemperateDeciduousForest,
                    p if p > 0.3 => BiomeType::TemperateGrassland,
                    p if p > 0.15 => BiomeType::Shrubland,
                    _ => BiomeType::HotDesert,
                }
            },
            _ => {
                // Hot climates (tropical)
                match precip {
                    p if p > 0.7 => BiomeType::TropicalRainforest,
                    p if p > 0.4 => BiomeType::TropicalSeasonalForest,
                    p if p > 0.2 => BiomeType::TropicalGrasslandSavanna,
                    p if p > 0.1 => BiomeType::Shrubland,
                    _ => BiomeType::HotDesert,
                }
            }
        }
    }

    fn refine_river_network(&mut self) {
        // Add more rivers in biomes that should have dense river networks
        let mut additional_rivers = 0;
        
        for (coord, &flow) in &self.flow_accumulation.clone() {
            let tile = &self.tiles[&coord];
            
            // Skip if already has a river
            if tile.has_river {
                continue;
            }
            
            // Biome-specific thresholds for additional rivers
            let biome_threshold = match BiomeType::from_u8(tile.biome) {
                BiomeType::TropicalRainforest => 2.0,
                BiomeType::TemperateRainforest => 2.5,
                BiomeType::TropicalSeasonalForest | BiomeType::TemperateDeciduousForest => 3.0,
                BiomeType::TemperateGrassland | BiomeType::TaigaBorealForest => 3.5,
                BiomeType::TemperateConiferForest => 4.0,
                BiomeType::TundraWet | BiomeType::Wetland => 3.0,
                BiomeType::Shrubland => 6.0,
                BiomeType::HotDesert | BiomeType::ColdDesert => 12.0,
                BiomeType::TundraBarren => 8.0,
                _ => 5.0,
            };
            
            if flow >= biome_threshold {
                self.tiles.get_mut(&coord).unwrap().has_river = true;
                additional_rivers += 1;
            }
        }
        
        if additional_rivers > 0 {
            println!("Added {} additional rivers based on biome characteristics", additional_rivers);
            // Recalculate river flow rates and edges
            self.calculate_river_flow_rates();
        }
    }

    fn place_lakes(&mut self) {
        // Find natural depressions for lakes based on flow accumulation
        let coords: Vec<HexCoord> = self.tiles.keys().cloned().collect();
        let mut lake_candidates = Vec::new();
        
        for coord in coords {
            let tile = &self.tiles[&coord];
            
            // Must be above sea level in a natural depression
            if tile.elevation > self.sea_level && tile.elevation < self.sea_level + 0.3 {
                let neighbors = coord.neighbors();
                let mut higher_neighbors = 0;
                let mut total_neighbor_elevation = 0.0;
                let mut neighbor_count = 0;
                
                for neighbor in neighbors {
                    if let Some(neighbor_tile) = self.tiles.get(&neighbor) {
                        if neighbor_tile.elevation > tile.elevation {
                            higher_neighbors += 1;
                        }
                        total_neighbor_elevation += neighbor_tile.elevation;
                        neighbor_count += 1;
                    }
                }
                
                let avg_neighbor_elevation = total_neighbor_elevation / neighbor_count as f32;
                let depression_depth = avg_neighbor_elevation - tile.elevation;
                
                // Check if this is a flow convergence point (multiple streams flowing in)
                let incoming_flows = neighbors.iter()
                    .filter(|&&neighbor| {
                        self.flow_directions.get(&neighbor)
                            .map(|(_, target)| *target == coord)
                            .unwrap_or(false)
                    })
                    .count();
                
                // Good lake candidate if surrounded by higher ground AND has incoming flow
                if higher_neighbors >= 4 && depression_depth > 0.05 && incoming_flows >= 2 {
                    lake_candidates.push((coord, depression_depth + incoming_flows as f32 * 0.1));
                }
            }
        }
        
        // Sort by score (depression depth + flow convergence)
        lake_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        let mut lakes_placed = 0;
        for (coord, _) in lake_candidates.iter().take(25) {
            // Ensure lakes are well-spaced
            let too_close = self.tiles.values()
                .filter(|t| t.biome == BiomeType::Lake as u8)
                .any(|lake_tile| self.hex_distance(*coord, lake_tile.hex_coord) < 6);
                
            if !too_close {
                self.tiles.get_mut(coord).unwrap().biome = BiomeType::Lake as u8;
                lakes_placed += 1;
            }
        }
        
        println!("Placed {} lakes at flow convergence points", lakes_placed);
    }

    fn calculate_soil_fertility(&mut self) {
        let coords: Vec<HexCoord> = self.tiles.keys().cloned().collect();
        
        for coord in coords {
            let tile = &self.tiles[&coord];
            
            let base_fertility = match BiomeType::from_u8(tile.biome) {
                BiomeType::TropicalRainforest => 0.6,       // Rich but leached
                BiomeType::TropicalSeasonalForest => 0.8,   // Very fertile
                BiomeType::TropicalGrasslandSavanna => 0.9, // Excellent for agriculture
                BiomeType::TemperateGrassland => 1.0,       // Prime farmland
                BiomeType::TemperateDeciduousForest => 0.7, // Good when cleared
                BiomeType::TemperateConiferForest => 0.4,   // Acidic soils
                BiomeType::TaigaBorealForest => 0.3,        // Cold, acidic
                BiomeType::Wetland => 0.8,                  // Rich when drained
                _ => 0.2, // Deserts, tundra, mountains
            };
            
            // River bonus (fertile floodplains)
            let river_bonus = if tile.has_river { 0.3 } else { 0.0 };
            
            // Geology modifier
            let geology_modifier = match GeologyType::from_u8(tile.geology) {
                GeologyType::Sedimentary => 0.2,     // Good agricultural soils
                GeologyType::Limestone => 0.1,       // Alkaline soils
                GeologyType::Volcanic => 0.3,        // Very fertile volcanic soils
                _ => 0.0,
            };
            
            let fertility = (base_fertility as f32 + river_bonus as f32 + geology_modifier as f32).min(1.0);
            
            // Update the tile
            self.tiles.get_mut(&coord).unwrap().soil_fertility = fertility;
        }
    }

    fn place_geological_resources(&mut self) {
        let coords: Vec<HexCoord> = self.tiles.keys().cloned().collect();
        
        for coord in coords {
            let tile = &self.tiles[&coord];
            let resource = self.generate_biome_resource(tile.hex_coord, tile.biome);
            
            // Update the tile
            self.tiles.get_mut(&coord).unwrap().resource = resource;
        }
    }

    fn generate_biome_resource(&self, hex_coord: HexCoord, biome: u8) -> u8 {
        use noise::{NoiseFn, Perlin};
        let resource_noise = Perlin::new(789);
        
        // Lower chance of resources (about 15% of tiles)
        let resource_chance = resource_noise.get([
            hex_coord.q as f64 * 0.3,
            hex_coord.r as f64 * 0.3,
        ]) as f32;
        
        if resource_chance > 0.7 {
            use super::resources::ResourceType;
            let possible_resources = match BiomeType::from_u8(biome) {
                BiomeType::Ocean | BiomeType::Lake | BiomeType::River => {
                    vec![ResourceType::Fish]
                },
                BiomeType::TemperateGrassland | BiomeType::TropicalGrasslandSavanna => {
                    vec![ResourceType::Wheat, ResourceType::Horses, ResourceType::Cattle]
                },
                BiomeType::AlpineTundra | BiomeType::MontaneForest => {
                    vec![ResourceType::Iron, ResourceType::Stone, ResourceType::Copper, ResourceType::Coal]
                },
                BiomeType::TemperateDeciduousForest | BiomeType::TemperateConiferForest | 
                BiomeType::TaigaBorealForest | BiomeType::TropicalRainforest | BiomeType::TropicalSeasonalForest => {
                    vec![ResourceType::Wood, ResourceType::Spices, ResourceType::Silk]
                },
                BiomeType::HotDesert | BiomeType::ColdDesert => {
                    vec![ResourceType::Oil, ResourceType::Gold, ResourceType::Gems]
                },
                BiomeType::TundraBarren | BiomeType::TundraWet => {
                    vec![ResourceType::Oil, ResourceType::Iron]
                },
                BiomeType::Mangrove | BiomeType::SaltMarsh => {
                    vec![ResourceType::Fish, ResourceType::Salt]
                },
                _ => vec![ResourceType::Stone],
            };
            
            if !possible_resources.is_empty() {
                // Use coordinate hash to pick consistent resource
                let index = ((hex_coord.q.abs() + hex_coord.r.abs() * 3) as usize) % possible_resources.len();
                return possible_resources[index] as u8;
            }
        }
        
        0 // No resource
    }

    fn place_biological_resources(&mut self) {
        // This could be expanded to place resources like game animals, medicinal plants, etc.
        // For now, the existing resource system handles this
    }

    // Helper functions
    fn distance_to_ocean(&self, coord: HexCoord) -> f32 {
        // Simple approximation - in a full implementation you'd use flood-fill
        let mut min_distance = f32::INFINITY;
        
        for tile in self.tiles.values() {
            if tile.elevation <= self.sea_level {
                let distance = self.hex_distance(coord, tile.hex_coord) as f32;
                min_distance = min_distance.min(distance);
            }
        }
        
        min_distance
    }

    fn hex_distance(&self, a: HexCoord, b: HexCoord) -> i32 {
        let dq = (a.q - b.q).abs();
        let dr = (a.r - b.r).abs();
        let ds = (-(a.q + a.r) + (b.q + b.r)).abs();
        (dq.max(dr)).max(ds)
    }

    fn step_in_direction(&self, coord: HexCoord, direction: usize) -> HexCoord {
        let directions = [
            (1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)
        ];
        let (dq, dr) = directions[direction % 6];
        HexCoord::new(coord.q + dq, coord.r + dr)
    }
}

// Helper trait for direction finding
trait HexDirection {
    fn direction_to(&self, other: HexCoord) -> Option<usize>;
}

impl HexDirection for HexCoord {
    fn direction_to(&self, other: HexCoord) -> Option<usize> {
        let directions = self.neighbors();
        directions.iter().position(|&n| n == other)
    }
}

// Helper for geology enum
impl GeologyType {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => GeologyType::OceanicCrust,
            1 => GeologyType::ContinentalShelf,
            2 => GeologyType::Sedimentary,
            3 => GeologyType::Igneous,
            4 => GeologyType::Metamorphic,
            5 => GeologyType::Volcanic,
            6 => GeologyType::Limestone,
            7 => GeologyType::Sandstone,
            8 => GeologyType::Granite,
            9 => GeologyType::Basalt,
            _ => GeologyType::Sedimentary,
        }
    }
}