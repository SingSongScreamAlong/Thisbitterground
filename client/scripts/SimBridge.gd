extends Node
class_name SimBridge

## SimBridge.gd
## Bridge between Godot and the Rust ECS simulation.
##
## Automatically uses Rust backend (SimWorldBridge) if available,
## otherwise falls back to GDScript mock implementation.

# Rust backend (loaded via GDExtension)
var _rust_bridge: RefCounted = null
var _use_rust: bool = false

# Mock simulation state (fallback when Rust not available)
var _mock_tick: int = 0
var _mock_time: float = 0.0
var _mock_squads: Array[Dictionary] = []
var _mock_terrain_damage: Array[Dictionary] = []
var _mock_tick_rate: float = 20.0
var _mock_tick_accumulator: float = 0.0

# Public accessors
var current_tick: int:
	get:
		if _use_rust and _rust_bridge:
			return _rust_bridge.get_current_tick()
		return _mock_tick

var current_time: float:
	get:
		if _use_rust and _rust_bridge:
			return _rust_bridge.get_current_time()
		return _mock_time

func _ready() -> void:
	_try_load_rust_backend()

func _try_load_rust_backend() -> void:
	# Try to instantiate the Rust SimWorldBridge class
	if ClassDB.class_exists("SimWorldBridge"):
		_rust_bridge = ClassDB.instantiate("SimWorldBridge")
		if _rust_bridge:
			_use_rust = true
			print("[SimBridge] Using Rust ECS backend")
			return
	print("[SimBridge] Rust backend not available, using GDScript mock")
	_use_rust = false

## Check if using Rust backend.
func is_using_rust() -> bool:
	return _use_rust

## Initialize the simulation world.
func init_world() -> void:
	if _use_rust and _rust_bridge:
		_rust_bridge.init_world()
		print("[SimBridge] Rust world initialized")
	else:
		_mock_tick = 0
		_mock_time = 0.0
		_mock_squads.clear()
		_mock_terrain_damage.clear()
		_spawn_test_squads()
		print("[SimBridge] Mock world initialized")

## Step the simulation forward by delta seconds.
func step(delta: float) -> void:
	if _use_rust and _rust_bridge:
		_rust_bridge.step(delta)
	else:
		_mock_tick_accumulator += delta
		var tick_delta := 1.0 / _mock_tick_rate
		while _mock_tick_accumulator >= tick_delta:
			_mock_tick_accumulator -= tick_delta
			_run_tick(tick_delta)

## Get the current simulation snapshot as a Dictionary.
func get_snapshot() -> Dictionary:
	if _use_rust and _rust_bridge:
		var json_str: String = _rust_bridge.get_snapshot_json()
		var json := JSON.new()
		var err := json.parse(json_str)
		if err == OK:
			return json.data
		else:
			push_error("[SimBridge] Failed to parse Rust snapshot JSON")
			return {}
	return {
		"tick": _mock_tick,
		"time": _mock_time,
		"squads": _mock_squads.duplicate(true),
		"terrain_damage": _mock_terrain_damage.duplicate(true)
	}

## Get the snapshot as a JSON string.
func get_snapshot_json() -> String:
	if _use_rust and _rust_bridge:
		return _rust_bridge.get_snapshot_json()
	return JSON.stringify(get_snapshot())

## Issue a move order to a squad.
func order_move(squad_id: int, target_x: float, target_y: float) -> void:
	if _use_rust and _rust_bridge:
		_rust_bridge.order_move(squad_id, target_x, target_y)
	else:
		for squad in _mock_squads:
			if squad["id"] == squad_id:
				squad["order"] = "MoveTo(%.1f,%.1f)" % [target_x, target_y]
				squad["target_x"] = target_x
				squad["target_y"] = target_y
				squad["order_type"] = "MoveTo"
				break

## Issue an attack-move order to a squad.
func order_attack_move(squad_id: int, target_x: float, target_y: float) -> void:
	if _use_rust and _rust_bridge:
		_rust_bridge.order_attack_move(squad_id, target_x, target_y)
	else:
		for squad in _mock_squads:
			if squad["id"] == squad_id:
				squad["order"] = "AttackMove(%.1f,%.1f)" % [target_x, target_y]
				squad["target_x"] = target_x
				squad["target_y"] = target_y
				squad["order_type"] = "AttackMove"
				break

## Issue a hold order to a squad.
func order_hold(squad_id: int) -> void:
	if _use_rust and _rust_bridge:
		_rust_bridge.order_hold(squad_id)
	else:
		for squad in _mock_squads:
			if squad["id"] == squad_id:
				squad["order"] = "Hold"
				squad["order_type"] = "Hold"
				break

## Spawn a terrain damage event (crater).
func spawn_crater(x: float, y: float, radius: float, depth: float) -> void:
	if _use_rust and _rust_bridge:
		_rust_bridge.spawn_crater(x, y, radius, depth)
	else:
		_mock_terrain_damage.append({
			"x": x,
			"y": y,
			"radius": radius,
			"depth": depth
		})
		_mock_craters.append({
			"x": x,
			"y": y,
			"radius": radius,
			"depth": depth,
			"age": 0.0
		})

## Spawn an artillery barrage.
func spawn_barrage(center_x: float, center_y: float, spread: float, count: int) -> void:
	if _use_rust and _rust_bridge:
		_rust_bridge.spawn_barrage(center_x, center_y, spread, count)
	else:
		# Mock barrage
		for i in range(count):
			var angle := (float(i) / float(count)) * TAU + float(i) * 1.618
			var dist := spread * (0.3 + 0.7 * abs(sin(float(i) * 0.7)))
			var x := center_x + dist * cos(angle)
			var y := center_y + dist * sin(angle)
			spawn_crater(x, y, 3.0 + spread * 0.1, 1.5)

## Get terrain snapshot as Dictionary.
func get_terrain() -> Dictionary:
	if _use_rust and _rust_bridge:
		var json_str: String = _rust_bridge.get_terrain_json()
		var json := JSON.new()
		var err := json.parse(json_str)
		if err == OK:
			return json.data
		return {}
	return _mock_terrain

## Get movement multiplier at a position.
func get_movement_multiplier(x: float, y: float) -> float:
	if _use_rust and _rust_bridge:
		return _rust_bridge.get_movement_multiplier(x, y)
	return _get_mock_movement_multiplier(x, y)

## Get cover value at a position.
func get_cover_at(x: float, y: float) -> float:
	if _use_rust and _rust_bridge:
		return _rust_bridge.get_cover_at(x, y)
	return _get_mock_cover_at(x, y)

## Get terrain height at a position.
func get_height_at(x: float, y: float) -> float:
	if _use_rust and _rust_bridge:
		return _rust_bridge.get_height_at(x, y)
	return _get_mock_height_at(x, y)

## Get all craters.
func get_craters() -> Array:
	if _use_rust and _rust_bridge:
		var terrain := get_terrain()
		return terrain.get("craters", [])
	return _mock_craters

# --- Internal mock simulation (fallback) ---

# Mock terrain data
var _mock_terrain: Dictionary = {}
var _mock_craters: Array = []

func _get_mock_movement_multiplier(x: float, y: float) -> float:
	# Check if in crater
	for crater in _mock_craters:
		var dx := x - crater["x"]
		var dy := y - crater["y"]
		var dist := sqrt(dx * dx + dy * dy)
		if dist < crater["radius"]:
			return 0.6  # Crater movement penalty
	return 1.0

func _get_mock_cover_at(x: float, y: float) -> float:
	# Check if in crater
	for crater in _mock_craters:
		var dx := x - crater["x"]
		var dy := y - crater["y"]
		var dist := sqrt(dx * dx + dy * dy)
		if dist < crater["radius"]:
			return 0.5  # Crater cover bonus
	return 0.0

func _get_mock_height_at(_x: float, _y: float) -> float:
	# Simple flat terrain for mock
	return 0.0

func _spawn_test_squads() -> void:
	# Blue faction on the left
	for i in range(6):
		_mock_squads.append({
			"id": i,
			"faction": "Blue",
			"x": -50.0,
			"y": -25.0 + float(i) * 10.0,
			"vx": 0.0,
			"vy": 0.0,
			"health": 100.0,
			"health_max": 100.0,
			"size": 12,
			"morale": 1.0,
			"suppression": 0.0,
			"order": "Hold",
			"order_type": "Hold",
			"target_x": 0.0,
			"target_y": 0.0,
			"speed": 5.0
		})
	
	# Red faction on the right
	for i in range(6):
		_mock_squads.append({
			"id": 100 + i,
			"faction": "Red",
			"x": 50.0,
			"y": -25.0 + float(i) * 10.0,
			"vx": 0.0,
			"vy": 0.0,
			"health": 100.0,
			"health_max": 100.0,
			"size": 12,
			"morale": 1.0,
			"suppression": 0.0,
			"order": "Hold",
			"order_type": "Hold",
			"target_x": 0.0,
			"target_y": 0.0,
			"speed": 5.0
		})

func _run_tick(delta: float) -> void:
	_mock_tick += 1
	_mock_time += delta
	
	# Process orders and movement
	for squad in _mock_squads:
		_process_squad_order(squad, delta)
		_process_squad_combat(squad, delta)
	
	# Decay suppression
	for squad in _mock_squads:
		squad["suppression"] = max(0.0, squad["suppression"] - 0.15 * delta)
	
	# Clear terrain damage after one tick (consumed by visualization)
	_mock_terrain_damage.clear()

func _process_squad_order(squad: Dictionary, delta: float) -> void:
	var order_type: String = squad.get("order_type", "Hold")
	
	# Skip if pinned or broken
	if squad["suppression"] >= 1.0 or squad["morale"] < 0.2:
		squad["vx"] = 0.0
		squad["vy"] = 0.0
		return
	
	match order_type:
		"Hold":
			squad["vx"] = 0.0
			squad["vy"] = 0.0
		"MoveTo", "AttackMove":
			var dx: float = squad["target_x"] - squad["x"]
			var dy: float = squad["target_y"] - squad["y"]
			var dist := sqrt(dx * dx + dy * dy)
			
			if dist < 1.0:
				# Arrived
				squad["vx"] = 0.0
				squad["vy"] = 0.0
				squad["order"] = "Hold"
				squad["order_type"] = "Hold"
			else:
				var speed: float = squad["speed"]
				if order_type == "AttackMove":
					speed *= 0.6
				squad["vx"] = (dx / dist) * speed
				squad["vy"] = (dy / dist) * speed
	
	# Apply velocity
	var speed_mult := 1.0
	if squad["suppression"] >= 0.5:
		speed_mult = 0.3
	elif squad["morale"] < 0.5:
		speed_mult = 0.6
	
	squad["x"] += squad["vx"] * delta * speed_mult
	squad["y"] += squad["vy"] * delta * speed_mult

func _process_squad_combat(squad: Dictionary, _delta: float) -> void:
	# Simple combat: apply suppression to enemies in range
	var fire_range := 60.0
	
	for other in _mock_squads:
		if other["id"] == squad["id"]:
			continue
		if other["faction"] == squad["faction"]:
			continue
		if other["health"] <= 0:
			continue
		
		var dx: float = other["x"] - squad["x"]
		var dy: float = other["y"] - squad["y"]
		var dist := sqrt(dx * dx + dy * dy)
		
		if dist <= fire_range:
			other["suppression"] += 0.05
