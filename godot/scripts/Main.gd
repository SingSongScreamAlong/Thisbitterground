extends Node2D
## Main scene controller for This Bitter Ground visualization.
##
## This script creates a RustSimulation instance, steps it each frame,
## and renders squad positions as simple colored circles.
##
## =============================================================================
## FLATBUFFER LAYOUT (from sim/src/godot_bridge.rs)
## =============================================================================
## The snapshot buffer returned by get_snapshot_buffer() has this layout:
##
## HEADER (1 float):
##   buffer[0] = squad_count
##
## PER-SQUAD DATA (14 floats each, SQUAD_STRIDE = 14):
##   For squad i at offset = 1 + i * 14:
##     [+0]  id          - Squad ID (u32 as f32)
##     [+1]  x           - X position (world units)
##     [+2]  y           - Y position (world units)
##     [+3]  vx          - X velocity
##     [+4]  vy          - Y velocity
##     [+5]  faction_id  - 0.0 = Blue, 1.0 = Red
##     [+6]  size        - Squad size (soldier count)
##     [+7]  health      - Current health
##     [+8]  health_max  - Maximum health
##     [+9]  morale      - Morale (0.0 - 1.0)
##     [+10] suppression - Suppression level (0.0 - 1.0)
##     [+11] is_alive    - 1.0 = alive, 0.0 = dead
##     [+12] is_routing  - 1.0 = routing/fleeing
##     [+13] order_type  - 0=Hold, 1=MoveTo, 2=AttackMove, 3=Retreat
##
## =============================================================================

# -----------------------------------------------------------------------------
# CONSTANTS
# -----------------------------------------------------------------------------

## Colors for faction visualization
const COLOR_BLUE := Color(0.2, 0.4, 0.9, 1.0)
const COLOR_RED := Color(0.9, 0.2, 0.2, 1.0)
const COLOR_DEAD := Color(0.3, 0.3, 0.3, 0.5)
const COLOR_ROUTING := Color(1.0, 0.8, 0.2, 1.0)

## Size of squad markers in pixels
const MARKER_RADIUS := 8.0

## World-to-screen scale (world units -> pixels)
const WORLD_SCALE := 2.0

## Field offsets within each squad's data block
const FIELD_ID := 0
const FIELD_X := 1
const FIELD_Y := 2
const FIELD_VX := 3
const FIELD_VY := 4
const FIELD_FACTION := 5
const FIELD_SIZE := 6
const FIELD_HEALTH := 7
const FIELD_HEALTH_MAX := 8
const FIELD_MORALE := 9
const FIELD_SUPPRESSION := 10
const FIELD_IS_ALIVE := 11
const FIELD_IS_ROUTING := 12
const FIELD_ORDER_TYPE := 13

# -----------------------------------------------------------------------------
# NODES
# -----------------------------------------------------------------------------

@onready var camera: Camera2D = $Camera2D
@onready var debug_label: Label = $CanvasLayer/DebugPanel/DebugLabel

# -----------------------------------------------------------------------------
# STATE
# -----------------------------------------------------------------------------

## The Rust simulation instance
var sim: RefCounted = null

## Pool of marker nodes, keyed by squad ID
var markers: Dictionary = {}

## Cached stride and header size from Rust
var squad_stride: int = 14
var header_size: int = 1

# -----------------------------------------------------------------------------
# LIFECYCLE
# -----------------------------------------------------------------------------

func _ready() -> void:
	# Create the Rust simulation
	sim = RustSimulation.new()
	
	if sim == null:
		push_error("Failed to create RustSimulation - is the GDExtension loaded?")
		return
	
	# Cache constants from Rust side
	squad_stride = sim.get_squad_stride()
	header_size = sim.get_header_size()
	
	print("[Main] RustSimulation created")
	print("[Main] Squad stride: ", squad_stride)
	print("[Main] Header size: ", header_size)
	
	# Spawn test squads: Blue on the left, Red on the right
	_spawn_test_battle()
	
	print("[Main] Test battle spawned")


func _spawn_test_battle() -> void:
	## Spawn a simple test scenario with two opposing forces.
	const FACTION_BLUE := 0
	const FACTION_RED := 1
	
	# Blue force on the left
	sim.spawn_mass_squads(FACTION_BLUE, -150.0, 0.0, 20, 200.0, 1)
	
	# Red force on the right
	sim.spawn_mass_squads(FACTION_RED, 150.0, 0.0, 20, 200.0, 100)
	
	# Issue attack-move orders toward each other
	for i in range(20):
		sim.issue_attack_move_order(1 + i, 150.0, 0.0)      # Blue -> right
		sim.issue_attack_move_order(100 + i, -150.0, 0.0)   # Red -> left


func _process(delta: float) -> void:
	if sim == null:
		return
	
	# Step the Rust simulation
	sim.step(delta)
	
	# Get the snapshot buffer
	var buffer: PackedFloat32Array = sim.get_snapshot_buffer()
	
	if buffer.size() < header_size:
		return
	
	# Parse and render squads
	var squad_count := int(buffer[0])
	_update_markers(buffer, squad_count)
	
	# Update debug display
	_update_debug_label(squad_count)


# -----------------------------------------------------------------------------
# RENDERING
# -----------------------------------------------------------------------------

func _update_markers(buffer: PackedFloat32Array, squad_count: int) -> void:
	## Update marker positions and visibility based on snapshot buffer.
	
	var seen_ids: Dictionary = {}
	
	for i in range(squad_count):
		var base := header_size + i * squad_stride
		
		# Bounds check
		if base + squad_stride > buffer.size():
			break
		
		# Extract squad data
		var squad_id := int(buffer[base + FIELD_ID])
		var x := buffer[base + FIELD_X]
		var y := buffer[base + FIELD_Y]
		var faction_id := int(buffer[base + FIELD_FACTION])
		var health := buffer[base + FIELD_HEALTH]
		var health_max := buffer[base + FIELD_HEALTH_MAX]
		var is_alive := buffer[base + FIELD_IS_ALIVE] > 0.5
		var is_routing := buffer[base + FIELD_IS_ROUTING] > 0.5
		
		seen_ids[squad_id] = true
		
		# Get or create marker
		var marker: Node2D
		if markers.has(squad_id):
			marker = markers[squad_id]
		else:
			marker = _create_marker(squad_id)
			markers[squad_id] = marker
		
		# Update position (convert world coords to screen coords)
		# Center the view: world origin at screen center
		var screen_pos := Vector2(x, -y) * WORLD_SCALE + Vector2(640, 360)
		marker.position = screen_pos
		
		# Update visibility and color
		marker.visible = true
		var color: Color
		if not is_alive:
			color = COLOR_DEAD
		elif is_routing:
			color = COLOR_ROUTING
		elif faction_id == 0:
			color = COLOR_BLUE
		else:
			color = COLOR_RED
		
		# Modulate based on health
		var health_ratio := health / max(health_max, 1.0)
		color.a = 0.5 + 0.5 * health_ratio
		
		marker.modulate = color
	
	# Hide markers for squads no longer in the buffer
	for squad_id in markers.keys():
		if not seen_ids.has(squad_id):
			markers[squad_id].visible = false


func _create_marker(squad_id: int) -> Node2D:
	## Create a simple circle marker for a squad.
	var marker := Node2D.new()
	marker.name = "Squad_%d" % squad_id
	
	# Add a simple circle using a ColorRect with rounded corners
	# For simplicity, we'll use _draw() on a custom node
	var circle := SquadMarker.new()
	circle.radius = MARKER_RADIUS
	marker.add_child(circle)
	
	add_child(marker)
	return marker


func _update_debug_label(squad_count: int) -> void:
	## Update the debug overlay with simulation stats.
	var tick := sim.get_tick()
	var sim_time := sim.get_time()
	var fps := Engine.get_frames_per_second()
	
	debug_label.text = "FPS: %d\nTick: %d\nTime: %.1fs\nSquads: %d" % [
		fps, tick, sim_time, squad_count
	]


# =============================================================================
# SQUAD MARKER - Simple circle drawer
# =============================================================================

class SquadMarker extends Node2D:
	var radius: float = 8.0
	
	func _draw() -> void:
		draw_circle(Vector2.ZERO, radius, Color.WHITE)
		draw_arc(Vector2.ZERO, radius, 0, TAU, 32, Color(0, 0, 0, 0.5), 1.5)
