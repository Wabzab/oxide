use bevy::prelude::*;


#[derive(Component)]
struct Person;

#[derive(Component)]
struct Name(String);

fn main() {
    App::new()
    .add_plugins(DefaultPlugins)
    .add_systems(Startup, (add_people, greet_people).chain())
    .run();
}

fn add_people(mut commands : Commands) {
    commands.spawn((Person, Name("Wabz".to_string())));
    commands.spawn((Person, Name("Floopy".to_string())));
    commands.spawn((Person, Name("Dwarf".to_string())));
}

fn greet_people(query: Query<&Name, With<Person>>) {
    for name in &query {
        println!("hello {}!", name.0);
    }
}
