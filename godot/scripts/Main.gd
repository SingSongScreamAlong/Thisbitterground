extends Node2D
## Main scene controller for This Bitter Ground - Interactive War Table.
##
## This script creates a RustSimulation instance, steps it each frame,
## renders squad positions, and handles camera controls, selection, and orders.
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
## CONTROLS
## =============================================================================
## Camera:
##   WASD / Arrow Keys - Pan camera
##   Mouse Wheel       - Zoom in/out
##
## Selection:
##   Left Click        - Select squad under cursor
##   Escape            - Deselect
##
## Orders (requires selected squad):
##   Right Click       - Move to position
##   Shift+Right Click - Attack-move to position
##   H                 - Hold position
##   R                 - Retreat
##
## =============================================================================

# -----------------------------------------------------------------------------
# CONSTANTS - VISUALS
# -----------------------------------------------------------------------------

## Colors for faction visualization
const COLOR_BLUE := Color(0.2, 0.4, 0.9, 1.0)
const COLOR_RED := Color(0.9, 0.2, 0.2, 1.0)
const COLOR_DEAD := Color(0.3, 0.3, 0.3, 0.5)
const COLOR_ROUTING := Color(1.0, 0.8, 0.2, 1.0)
const COLOR_SELECTED := Color(0.0, 1.0, 0.5, 1.0)  # Selection highlight

## Size of squad markers in pixels
const MARKER_RADIUS := 8.0
const SELECTION_RADIUS := 12.0  # Slightly larger for selection ring

## Selection hit detection radius (in screen pixels)
const SELECTION_HIT_RADIUS := 20.0

# -----------------------------------------------------------------------------
# CONSTANTS - CAMERA
# -----------------------------------------------------------------------------

## Camera pan speed in pixels per second
const CAMERA_PAN_SPEED := 400.0

## Zoom limits and speed
const ZOOM_MIN := 0.25
const ZOOM_MAX := 4.0
const ZOOM_STEP := 0.1

# -----------------------------------------------------------------------------
# CONSTANTS - WORLD TRANSFORM
# -----------------------------------------------------------------------------

## World-to-screen scale (world units -> pixels at zoom 1.0)
const WORLD_SCALE := 2.0

## Screen center offset (viewport center)
const SCREEN_CENTER := Vector2(640, 360)

# -----------------------------------------------------------------------------
# CONSTANTS - BUFFER FIELD OFFSETS
# -----------------------------------------------------------------------------

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

## Order type names for display
const ORDER_NAMES := ["Hold", "MoveTo", "AttackMove", "Retreat"]

# -----------------------------------------------------------------------------
# NODES
# -----------------------------------------------------------------------------

@onready var camera: Camera2D = $Camera2D
@onready var debug_label: Label = $CanvasLayer/DebugPanel/DebugLabel
@onready var selection_label: Label = $CanvasLayer/SelectionPanel/SelectionLabel

# -----------------------------------------------------------------------------
# STATE - SIMULATION
# -----------------------------------------------------------------------------

## The Rust simulation instance
var sim: RefCounted = null

## Pool of marker nodes, keyed by squad ID
var markers: Dictionary = {}

## Cached squad data from last buffer read, keyed by squad ID
## Each entry is a Dictionary with: id, x, y, vx, vy, faction, size, health, 
## health_max, morale, suppression, is_alive, is_routing, order_type
var squad_data_cache: Dictionary = {}

## Cached stride and header size from Rust
var squad_stride: int = 14
var header_size: int = 1

# -----------------------------------------------------------------------------
# STATE - SELECTION
# -----------------------------------------------------------------------------

## Currently selected squad ID, or -1 if none
var selected_squad_id: int = -1

## Selection ring node (drawn around selected squad)
var selection_ring: Node2D = null

# -----------------------------------------------------------------------------
# STATE - CAMERA
# -----------------------------------------------------------------------------

## Current camera zoom level
var camera_zoom: float = 1.0

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
	
	# Create selection ring (hidden initially)
	_create_selection_ring()
	
	print("[Main] RustSimulation created")
	print("[Main] Squad stride: ", squad_stride)
	print("[Main] Header size: ", header_size)
	
	# Spawn test squads: Blue on the left, Red on the right
	_spawn_test_battle()
	
	print("[Main] Test battle spawned - Click to select, right-click to move!")


func _spawn_test_battle() -> void:
	## Spawn a simple test scenario with two opposing forces.
	## Squads start with Hold orders so the player can issue commands.
	const FACTION_BLUE := 0
	const FACTION_RED := 1
	
	# Blue force on the left
	sim.spawn_mass_squads(FACTION_BLUE, -150.0, 0.0, 15, 180.0, 1)
	
	# Red force on the right  
	sim.spawn_mass_squads(FACTION_RED, 150.0, 0.0, 15, 180.0, 100)
	
	# Don't auto-issue orders - let the player control them!
	# (Squads start with Hold order by default)


func _process(delta: float) -> void:
	if sim == null:
		return
	
	# Handle camera controls
	_handle_camera_input(delta)
	
	# Step the Rust simulation
	sim.step(delta)
	
	# Get the snapshot buffer and update visuals
	var buffer: PackedFloat32Array = sim.get_snapshot_buffer()
	if buffer.size() >= header_size:
		var squad_count := int(buffer[0])
		_update_squads_from_buffer(buffer, squad_count)
		_update_debug_label(squad_count)
	
	# Update selection visuals
	_update_selection_visuals()


func _unhandled_input(event: InputEvent) -> void:
	if sim == null:
		return
	
	# Mouse button events
	if event is InputEventMouseButton:
		_handle_mouse_button(event)
	
	# Keyboard shortcuts for orders
	if event is InputEventKey and event.pressed:
		_handle_keyboard_orders(event)


# -----------------------------------------------------------------------------
# CAMERA CONTROLS
# -----------------------------------------------------------------------------

func _handle_camera_input(delta: float) -> void:
	## Handle WASD/arrow key panning and mouse wheel zoom.
	
	# Pan direction from input
	var pan_dir := Vector2.ZERO
	
	if Input.is_action_pressed("ui_left") or Input.is_key_pressed(KEY_A):
		pan_dir.x -= 1
	if Input.is_action_pressed("ui_right") or Input.is_key_pressed(KEY_D):
		pan_dir.x += 1
	if Input.is_action_pressed("ui_up") or Input.is_key_pressed(KEY_W):
		pan_dir.y -= 1
	if Input.is_action_pressed("ui_down") or Input.is_key_pressed(KEY_S):
		pan_dir.y += 1
	
	# Apply pan (frame-rate independent, adjusted for zoom)
	if pan_dir != Vector2.ZERO:
		pan_dir = pan_dir.normalized()
		camera.position += pan_dir * CAMERA_PAN_SPEED * delta / camera_zoom


func _handle_mouse_wheel(event: InputEventMouseButton) -> void:
	## Handle zoom in/out with mouse wheel.
	
	if event.button_index == MOUSE_BUTTON_WHEEL_UP:
		camera_zoom = clampf(camera_zoom + ZOOM_STEP, ZOOM_MIN, ZOOM_MAX)
	elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
		camera_zoom = clampf(camera_zoom - ZOOM_STEP, ZOOM_MIN, ZOOM_MAX)
	
	camera.zoom = Vector2(camera_zoom, camera_zoom)


# -----------------------------------------------------------------------------
# INPUT HANDLING
# -----------------------------------------------------------------------------

func _handle_mouse_button(event: InputEventMouseButton) -> void:
	## Handle mouse clicks for selection and orders.
	
	# Zoom with mouse wheel
	if event.button_index in [MOUSE_BUTTON_WHEEL_UP, MOUSE_BUTTON_WHEEL_DOWN]:
		_handle_mouse_wheel(event)
		return
	
	if not event.pressed:
		return
	
	# Left click = select squad
	if event.button_index == MOUSE_BUTTON_LEFT:
		_select_squad_at_mouse(event.position)
	
	# Right click = issue order to selected squad
	elif event.button_index == MOUSE_BUTTON_RIGHT:
		if selected_squad_id >= 0:
			var world_pos := _screen_to_world(event.position)
			var use_attack_move := Input.is_key_pressed(KEY_SHIFT)
			_issue_order_to_selected(world_pos, use_attack_move)


func _handle_keyboard_orders(event: InputEventKey) -> void:
	## Handle keyboard shortcuts for orders.
	
	if selected_squad_id < 0:
		return  # No squad selected
	
	match event.keycode:
		KEY_H:
			# Hold position
			sim.issue_hold_order(selected_squad_id)
			print("[Order] Squad %d: Hold" % selected_squad_id)
		KEY_R:
			# Retreat
			sim.issue_retreat_order(selected_squad_id)
			print("[Order] Squad %d: Retreat" % selected_squad_id)
		KEY_ESCAPE:
			# Deselect
			selected_squad_id = -1
			print("[Selection] Deselected")


# -----------------------------------------------------------------------------
# SELECTION
# -----------------------------------------------------------------------------

func _select_squad_at_mouse(screen_pos: Vector2) -> void:
	## Find and select the closest squad to the mouse position.
	## Uses screen-space distance for hit detection.
	
	var best_id := -1
	var best_dist := SELECTION_HIT_RADIUS
	
	# Iterate through all markers and find closest to click
	for squad_id in markers.keys():
		var marker: Node2D = markers[squad_id]
		if not marker.visible:
			continue
		
		# Check if squad is alive
		if squad_data_cache.has(squad_id):
			var data: Dictionary = squad_data_cache[squad_id]
			if not data.get("is_alive", false):
				continue
		
		# Get marker screen position (accounting for camera)
		var marker_screen_pos := _world_to_screen_with_camera(marker.position)
		var dist := screen_pos.distance_to(marker_screen_pos)
		
		if dist < best_dist:
			best_dist = dist
			best_id = squad_id
	
	# Update selection
	if best_id != selected_squad_id:
		selected_squad_id = best_id
		if best_id >= 0:
			print("[Selection] Selected squad %d" % best_id)
		else:
			print("[Selection] Deselected (no squad at click)")


func _world_to_screen_with_camera(marker_pos: Vector2) -> Vector2:
	## Convert a marker's position to screen coordinates, accounting for camera.
	## Markers are positioned in "screen space" relative to SCREEN_CENTER,
	## but we need to account for camera pan and zoom.
	
	# marker_pos is already in screen-ish coords (centered at SCREEN_CENTER)
	# Adjust for camera offset and zoom
	var offset := marker_pos - camera.position
	return SCREEN_CENTER + offset * camera_zoom


# -----------------------------------------------------------------------------
# ORDER ISSUING
# -----------------------------------------------------------------------------

func _issue_order_to_selected(world_pos: Vector2, attack_move: bool) -> void:
	## Issue a move or attack-move order to the selected squad.
	## world_pos is in simulation world coordinates.
	
	if selected_squad_id < 0:
		return
	
	if attack_move:
		sim.issue_attack_move_order(selected_squad_id, world_pos.x, world_pos.y)
		print("[Order] Squad %d: Attack-move to (%.1f, %.1f)" % [
			selected_squad_id, world_pos.x, world_pos.y
		])
	else:
		sim.issue_move_order(selected_squad_id, world_pos.x, world_pos.y)
		print("[Order] Squad %d: Move to (%.1f, %.1f)" % [
			selected_squad_id, world_pos.x, world_pos.y
		])


# -----------------------------------------------------------------------------
# COORDINATE TRANSFORMS
# -----------------------------------------------------------------------------

func _world_to_screen(world_pos: Vector2) -> Vector2:
	## Convert simulation world coordinates to screen position.
	## Note: Y is flipped (world Y+ is up, screen Y+ is down).
	return Vector2(world_pos.x, -world_pos.y) * WORLD_SCALE + SCREEN_CENTER


func _screen_to_world(screen_pos: Vector2) -> Vector2:
	## Convert screen position to simulation world coordinates.
	## Accounts for camera pan and zoom.
	
	# First, convert screen pos to world-space marker position
	# (accounting for camera offset and zoom)
	var relative := (screen_pos - SCREEN_CENTER) / camera_zoom + camera.position - SCREEN_CENTER
	
	# Then convert from screen-space to world coordinates
	# Undo the SCREEN_CENTER offset and WORLD_SCALE, flip Y
	var world_x := relative.x / WORLD_SCALE
	var world_y := -relative.y / WORLD_SCALE
	
	return Vector2(world_x, world_y)


# -----------------------------------------------------------------------------
# SQUAD DATA EXTRACTION
# -----------------------------------------------------------------------------

func _get_squad_data(buffer: PackedFloat32Array, index: int) -> Dictionary:
	## Extract squad data from buffer at given index.
	## Returns a Dictionary with all squad fields.
	
	var base := header_size + index * squad_stride
	
	if base + squad_stride > buffer.size():
		return {}
	
	return {
		"id": int(buffer[base + FIELD_ID]),
		"x": buffer[base + FIELD_X],
		"y": buffer[base + FIELD_Y],
		"vx": buffer[base + FIELD_VX],
		"vy": buffer[base + FIELD_VY],
		"faction": int(buffer[base + FIELD_FACTION]),
		"size": int(buffer[base + FIELD_SIZE]),
		"health": buffer[base + FIELD_HEALTH],
		"health_max": buffer[base + FIELD_HEALTH_MAX],
		"morale": buffer[base + FIELD_MORALE],
		"suppression": buffer[base + FIELD_SUPPRESSION],
		"is_alive": buffer[base + FIELD_IS_ALIVE] > 0.5,
		"is_routing": buffer[base + FIELD_IS_ROUTING] > 0.5,
		"order_type": int(buffer[base + FIELD_ORDER_TYPE]),
	}


# -----------------------------------------------------------------------------
# RENDERING - SQUADS
# -----------------------------------------------------------------------------

func _update_squads_from_buffer(buffer: PackedFloat32Array, squad_count: int) -> void:
	## Update all squad markers and cache data from the snapshot buffer.
	
	var seen_ids: Dictionary = {}
	squad_data_cache.clear()
	
	for i in range(squad_count):
		var data := _get_squad_data(buffer, i)
		if data.is_empty():
			continue
		
		var squad_id: int = data["id"]
		seen_ids[squad_id] = true
		squad_data_cache[squad_id] = data
		
		# Get or create marker
		var marker: Node2D
		if markers.has(squad_id):
			marker = markers[squad_id]
		else:
			marker = _create_marker(squad_id)
			markers[squad_id] = marker
		
		# Update position (convert world coords to screen coords)
		marker.position = _world_to_screen(Vector2(data["x"], data["y"]))
		
		# Update visibility and color
		marker.visible = true
		var color := _get_squad_color(data)
		marker.modulate = color
	
	# Hide markers for squads no longer in the buffer
	for squad_id in markers.keys():
		if not seen_ids.has(squad_id):
			markers[squad_id].visible = false


func _get_squad_color(data: Dictionary) -> Color:
	## Determine the color for a squad based on its state.
	
	var color: Color
	
	if not data["is_alive"]:
		color = COLOR_DEAD
	elif data["is_routing"]:
		color = COLOR_ROUTING
	elif data["faction"] == 0:
		color = COLOR_BLUE
	else:
		color = COLOR_RED
	
	# Modulate alpha based on health
	var health_ratio: float = data["health"] / max(data["health_max"], 1.0)
	color.a = 0.5 + 0.5 * health_ratio
	
	return color


func _create_marker(squad_id: int) -> Node2D:
	## Create a simple circle marker for a squad.
	var marker := Node2D.new()
	marker.name = "Squad_%d" % squad_id
	
	var circle := SquadMarker.new()
	circle.radius = MARKER_RADIUS
	marker.add_child(circle)
	
	add_child(marker)
	return marker


# -----------------------------------------------------------------------------
# RENDERING - SELECTION
# -----------------------------------------------------------------------------

func _create_selection_ring() -> void:
	## Create the selection ring node (reused for selected squad).
	selection_ring = Node2D.new()
	selection_ring.name = "SelectionRing"
	selection_ring.visible = false
	
	var ring := SelectionRing.new()
	ring.radius = SELECTION_RADIUS
	selection_ring.add_child(ring)
	
	add_child(selection_ring)


func _update_selection_visuals() -> void:
	## Update selection ring position and visibility.
	
	if selected_squad_id < 0 or not markers.has(selected_squad_id):
		selection_ring.visible = false
		_update_selection_label(null)
		return
	
	var marker: Node2D = markers[selected_squad_id]
	if not marker.visible:
		selection_ring.visible = false
		_update_selection_label(null)
		return
	
	# Position ring on selected squad
	selection_ring.position = marker.position
	selection_ring.visible = true
	
	# Update selection info panel
	if squad_data_cache.has(selected_squad_id):
		_update_selection_label(squad_data_cache[selected_squad_id])
	else:
		_update_selection_label(null)


# -----------------------------------------------------------------------------
# UI UPDATES
# -----------------------------------------------------------------------------

func _update_debug_label(squad_count: int) -> void:
	## Update the debug overlay with simulation stats.
	var tick := sim.get_tick()
	var sim_time := sim.get_time()
	var fps := Engine.get_frames_per_second()
	
	debug_label.text = "FPS: %d | Tick: %d | Time: %.1fs | Squads: %d\nZoom: %.0f%%" % [
		fps, tick, sim_time, squad_count, camera_zoom * 100
	]


func _update_selection_label(data) -> void:
	## Update the selection info panel.
	
	if data == null:
		selection_label.text = "No selection\n\nClick a squad to select\nRight-click to move"
		return
	
	var faction_name := "Blue" if data["faction"] == 0 else "Red"
	var order_name := ORDER_NAMES[data["order_type"]] if data["order_type"] < ORDER_NAMES.size() else "Unknown"
	var status := "DEAD" if not data["is_alive"] else ("ROUTING" if data["is_routing"] else "Active")
	
	selection_label.text = """Squad #%d (%s)
Status: %s
Order: %s
Health: %.0f / %.0f
Morale: %.0f%%
Suppression: %.0f%%

[H] Hold  [R] Retreat
Right-click: Move
Shift+Right-click: Attack""" % [
		data["id"],
		faction_name,
		status,
		order_name,
		data["health"],
		data["health_max"],
		data["morale"] * 100,
		data["suppression"] * 100,
	]


# =============================================================================
# INNER CLASSES
# =============================================================================

## Simple circle drawer for squad markers
class SquadMarker extends Node2D:
	var radius: float = 8.0
	
	func _draw() -> void:
		draw_circle(Vector2.ZERO, radius, Color.WHITE)
		draw_arc(Vector2.ZERO, radius, 0, TAU, 32, Color(0, 0, 0, 0.5), 1.5)


## Selection ring drawer
class SelectionRing extends Node2D:
	var radius: float = 12.0
	
	func _draw() -> void:
		# Animated pulsing ring
		var pulse := 1.0 + 0.1 * sin(Time.get_ticks_msec() * 0.005)
		var r := radius * pulse
		draw_arc(Vector2.ZERO, r, 0, TAU, 32, Color(0, 1, 0.5, 0.9), 2.5)
		draw_arc(Vector2.ZERO, r + 2, 0, TAU, 32, Color(0, 1, 0.5, 0.4), 1.5)
	
	func _process(_delta: float) -> void:
		queue_redraw()  # Redraw each frame for animation
