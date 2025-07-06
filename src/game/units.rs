use bevy::prelude::*;

#[derive(Component)]
pub struct Unit {
    pub unit_type: UnitType,
    pub movement_points: u32,
    pub health: u32,
    pub civilization_id: u32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum UnitType {
    Warrior,
    Archer,
    Settler,
    Worker,
}
