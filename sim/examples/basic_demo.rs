//! Basic demonstration of the This Bitter Ground simulation.
//!
//! Run with: cargo run --example basic_demo

use tbg_sim::SimWorld;

fn main() {
    println!("=== This Bitter Ground - Simulation Demo ===\n");

    // Create a test world with squads
    let mut sim = SimWorld::new_default_test_world();

    println!("Initial state:");
    print_snapshot(&mut sim);

    // Issue move orders to Blue squads
    println!("\n--- Issuing move orders to Blue squads ---\n");
    for i in 0..6 {
        sim.order_move(i, 0.0, -25.0 + (i as f32) * 10.0);
    }

    // Issue move orders to Red squads
    for i in 0..6 {
        sim.order_attack_move(100 + i, 0.0, -25.0 + (i as f32) * 10.0);
    }

    // Run simulation for 200 ticks (10 seconds - enough for combat)
    println!("Running simulation for 200 ticks (10 seconds at 20 ticks/sec)...\n");
    for tick in 0..200 {
        sim.step(0.05); // 50ms per tick = 20 ticks/sec

        // Print state every 20 ticks
        if (tick + 1) % 20 == 0 {
            println!("--- Tick {} (t={:.1}s) ---", sim.current_tick(), sim.current_time());
            print_snapshot(&mut sim);
        }
    }

    // Spawn a crater
    println!("\n--- Spawning artillery crater at (0, 0) ---\n");
    sim.spawn_crater(0.0, 0.0, 10.0, 2.0);

    // Final snapshot as JSON
    println!("\n=== Final State (JSON) ===\n");
    println!("{}", sim.snapshot().to_json_pretty().unwrap());
}

fn print_snapshot(sim: &mut SimWorld) {
    let snapshot = sim.snapshot();
    
    println!("  Blue squads:");
    for squad in snapshot.squads.iter().filter(|s| s.faction == "Blue") {
        println!(
            "    Squad {}: pos=({:.1}, {:.1}) hp={:.0} morale={:.2} sup={:.2} [{}]",
            squad.id, squad.x, squad.y, squad.health, squad.morale, squad.suppression, squad.order
        );
    }
    
    println!("  Red squads:");
    for squad in snapshot.squads.iter().filter(|s| s.faction == "Red") {
        println!(
            "    Squad {}: pos=({:.1}, {:.1}) hp={:.0} morale={:.2} sup={:.2} [{}]",
            squad.id, squad.x, squad.y, squad.health, squad.morale, squad.suppression, squad.order
        );
    }
}
