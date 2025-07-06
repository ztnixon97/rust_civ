use bevy::prelude::*;

#[derive(Component)]
pub struct City {
    pub name: String,
    pub population: u32,
    pub food: f32,
    pub production: f32,
    pub science: f32,
}
