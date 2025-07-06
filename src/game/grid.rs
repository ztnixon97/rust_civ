use bevy::prelude::*;
use super::hex::HexCoord;
use super::map::HEX_SIZE;
use std::f32::consts::PI;

#[derive(Resource)]
pub struct GridSettings {
    pub show_grid: bool,
    pub grid_color: Color,
    pub grid_width: f32,
}

impl Default for GridSettings {
    fn default() -> Self {
        Self {
            show_grid: true, // Start with grid ON so we can see it
            grid_color: Color::srgba(1.0, 1.0, 1.0, 0.4), // More visible
            grid_width: 2.0, // Thicker lines
        }
    }
}

#[derive(Component)]
pub struct GridLine;

#[derive(Resource)]
pub struct GridAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
}

pub fn setup_grid_lines(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    grid_settings: Res<GridSettings>,
) {
    println!("Setting up grid lines...");
    
    let line_mesh = create_hex_outline_mesh(HEX_SIZE);
    let mesh_handle = meshes.add(line_mesh);
    let material_handle = materials.add(ColorMaterial::from(grid_settings.grid_color));

    // Store grid assets
    commands.insert_resource(GridAssets {
        mesh: mesh_handle.clone(),
        material: material_handle.clone(),
    });

    // Generate grid lines for the same area as the map
    let map_radius = crate::game::map::MAP_RADIUS;
    let mut grid_lines_created = 0;
    
    for q in -map_radius..=map_radius {
        for r in (-map_radius).max(-q - map_radius)..=(map_radius).min(-q + map_radius) {
            let hex_coord = HexCoord::new(q, r);
            let world_pos = hex_coord.to_world_pos(HEX_SIZE);

            let visibility = if grid_settings.show_grid {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };

            commands.spawn((
                GridLine,
                Mesh2d(mesh_handle.clone()),
                MeshMaterial2d(material_handle.clone()),
                Transform::from_translation(Vec3::new(world_pos.x, world_pos.y, 0.5)), // Above tiles
                visibility,
            ));
            
            grid_lines_created += 1;
        }
    }
    
    println!("Created {} grid lines", grid_lines_created);
}

pub fn toggle_grid_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut grid_settings: ResMut<GridSettings>,
    mut grid_query: Query<&mut Visibility, With<GridLine>>,
) {
    if keyboard.just_pressed(KeyCode::KeyG) {
        grid_settings.show_grid = !grid_settings.show_grid;
        
        let visibility = if grid_settings.show_grid {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        
        for mut vis in grid_query.iter_mut() {
            *vis = visibility;
        }
        
        println!("Grid lines: {}", if grid_settings.show_grid { "ON" } else { "OFF" });
    }
}

fn create_hex_outline_mesh(size: f32) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    
    // Create thicker hex outline by making it a thin ring instead of just lines
    let outer_size = size;
    let inner_size = size * 0.95; // Slightly smaller inner ring
    
    // Outer ring vertices
    for i in 0..6 {
        let angle = PI / 3.0 * i as f32 + PI / 6.0;
        let x_outer = outer_size * angle.cos();
        let y_outer = outer_size * angle.sin();
        let x_inner = inner_size * angle.cos();
        let y_inner = inner_size * angle.sin();
        
        vertices.push([x_outer, y_outer, 0.0]); // Outer vertex
        vertices.push([x_inner, y_inner, 0.0]); // Inner vertex
    }
    
    // Create triangles for the ring
    for i in 0..6 {
        let current_outer = i * 2;
        let current_inner = i * 2 + 1;
        let next_outer = ((i + 1) % 6) * 2;
        let next_inner = ((i + 1) % 6) * 2 + 1;
        
        // Two triangles per segment
        indices.extend_from_slice(&[
            current_outer, next_outer, current_inner,
            current_inner, next_outer, next_inner,
        ]);
    }
    
    Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_indices(bevy::render::mesh::Indices::U32(indices))
}