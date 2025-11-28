extends Node3D
class_name CameraController

## CameraController.gd
## Handles camera movement, zoom, and war-table view transitions.
## Features smooth interpolation, edge panning, and multiple view modes.

signal zoom_changed(zoom_level: float)
signal view_mode_changed(mode: String)
signal camera_moved(position: Vector3)

# Pan settings
@export_group("Pan")
@export var pan_speed: float = 50.0
@export var edge_pan_margin: int = 20  # Pixels from screen edge
@export var edge_pan_enabled: bool = true

# Zoom settings
@export_group("Zoom")
@export var zoom_speed: float = 8.0
@export var zoom_smooth_speed: float = 10.0  # Interpolation speed
@export var min_zoom: float = 15.0
@export var max_zoom: float = 250.0
@export var zoom_angle_near: float = 35.0  # Degrees from horizontal when zoomed in
@export var zoom_angle_far: float = 85.0   # Degrees from horizontal when zoomed out

# Bounds
@export_group("Bounds")
@export var bounds_enabled: bool = true
@export var bounds_min: Vector2 = Vector2(-200, -200)
@export var bounds_max: Vector2 = Vector2(200, 200)

# Current state
var current_zoom: float = 80.0
var target_zoom: float = 80.0
var camera: Camera3D

# View modes
enum ViewMode { TACTICAL, WAR_TABLE, CINEMATIC }
var current_view_mode: ViewMode = ViewMode.TACTICAL
var target_view_mode: ViewMode = ViewMode.TACTICAL
var view_transition_progress: float = 1.0
var view_transition_speed: float = 3.0

# Smooth movement
var target_position: Vector3 = Vector3.ZERO
var position_smooth_speed: float = 8.0

# Mouse drag
var is_dragging: bool = false
var drag_start_mouse: Vector2
var drag_start_position: Vector3

func _ready() -> void:
	camera = get_node_or_null("Camera3D")
	if camera == null:
		push_warning("[CameraController] No Camera3D child found")
	target_position = global_position
	_update_camera_transform_immediate()

func _process(delta: float) -> void:
	_handle_pan_input(delta)
	_handle_edge_pan(delta)
	_update_smooth_movement(delta)
	_update_view_transition(delta)

func _unhandled_input(event: InputEvent) -> void:
	# Mouse wheel zoom
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP and event.pressed:
			zoom_in()
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN and event.pressed:
			zoom_out()
		# Middle mouse drag
		elif event.button_index == MOUSE_BUTTON_MIDDLE:
			if event.pressed:
				is_dragging = true
				drag_start_mouse = event.position
				drag_start_position = target_position
			else:
				is_dragging = false
	
	# Mouse drag movement
	if event is InputEventMouseMotion and is_dragging:
		var drag_delta: Vector2 = event.position - drag_start_mouse
		var zoom_factor := current_zoom / 80.0
		var world_delta := Vector3(-drag_delta.x, 0, -drag_delta.y) * 0.1 * zoom_factor
		target_position = drag_start_position + world_delta
		_clamp_position()
	
	# View mode keys
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_TAB:
				cycle_view_mode()
			KEY_1:
				set_view_mode(ViewMode.TACTICAL)
			KEY_2:
				set_view_mode(ViewMode.WAR_TABLE)
			KEY_3:
				set_view_mode(ViewMode.CINEMATIC)
			KEY_HOME:
				reset_camera()

func _handle_pan_input(delta: float) -> void:
	if is_dragging:
		return
	
	var move_dir := Vector3.ZERO
	
	if Input.is_action_pressed("camera_pan_up"):
		move_dir.z -= 1.0
	if Input.is_action_pressed("camera_pan_down"):
		move_dir.z += 1.0
	if Input.is_action_pressed("camera_pan_left"):
		move_dir.x -= 1.0
	if Input.is_action_pressed("camera_pan_right"):
		move_dir.x += 1.0
	
	if move_dir.length() > 0:
		move_dir = move_dir.normalized()
		var zoom_factor := current_zoom / 80.0
		target_position += move_dir * pan_speed * zoom_factor * delta
		_clamp_position()

func _handle_edge_pan(delta: float) -> void:
	if not edge_pan_enabled or is_dragging:
		return
	
	var viewport := get_viewport()
	if viewport == null:
		return
	
	var mouse_pos := viewport.get_mouse_position()
	var screen_size := viewport.get_visible_rect().size
	var move_dir := Vector3.ZERO
	
	if mouse_pos.x < edge_pan_margin:
		move_dir.x -= 1.0
	elif mouse_pos.x > screen_size.x - edge_pan_margin:
		move_dir.x += 1.0
	
	if mouse_pos.y < edge_pan_margin:
		move_dir.z -= 1.0
	elif mouse_pos.y > screen_size.y - edge_pan_margin:
		move_dir.z += 1.0
	
	if move_dir.length() > 0:
		move_dir = move_dir.normalized()
		var zoom_factor := current_zoom / 80.0
		target_position += move_dir * pan_speed * zoom_factor * delta * 0.7
		_clamp_position()

func _update_smooth_movement(delta: float) -> void:
	# Smooth zoom
	if abs(current_zoom - target_zoom) > 0.01:
		current_zoom = lerp(current_zoom, target_zoom, zoom_smooth_speed * delta)
		_update_camera_transform()
	
	# Smooth position
	if global_position.distance_to(target_position) > 0.01:
		global_position = global_position.lerp(target_position, position_smooth_speed * delta)
		emit_signal("camera_moved", global_position)

func _update_view_transition(delta: float) -> void:
	if view_transition_progress < 1.0:
		view_transition_progress = min(1.0, view_transition_progress + view_transition_speed * delta)
		_update_camera_transform()

func _clamp_position() -> void:
	if bounds_enabled:
		target_position.x = clamp(target_position.x, bounds_min.x, bounds_max.x)
		target_position.z = clamp(target_position.z, bounds_min.y, bounds_max.y)

func zoom_in() -> void:
	set_zoom(target_zoom - zoom_speed)

func zoom_out() -> void:
	set_zoom(target_zoom + zoom_speed)

func set_zoom(new_zoom: float) -> void:
	target_zoom = clamp(new_zoom, min_zoom, max_zoom)
	emit_signal("zoom_changed", target_zoom)

func cycle_view_mode() -> void:
	match current_view_mode:
		ViewMode.TACTICAL:
			set_view_mode(ViewMode.WAR_TABLE)
		ViewMode.WAR_TABLE:
			set_view_mode(ViewMode.CINEMATIC)
		ViewMode.CINEMATIC:
			set_view_mode(ViewMode.TACTICAL)

func set_view_mode(mode: ViewMode) -> void:
	if mode == current_view_mode:
		return
	current_view_mode = mode
	view_transition_progress = 0.0
	var mode_name := _get_view_mode_name(mode)
	emit_signal("view_mode_changed", mode_name)
	print("[Camera] View mode: %s" % mode_name)

func _get_view_mode_name(mode: ViewMode) -> String:
	match mode:
		ViewMode.TACTICAL:
			return "TACTICAL"
		ViewMode.WAR_TABLE:
			return "WAR_TABLE"
		ViewMode.CINEMATIC:
			return "CINEMATIC"
	return "UNKNOWN"

func reset_camera() -> void:
	target_position = Vector3.ZERO
	target_zoom = 80.0
	set_view_mode(ViewMode.TACTICAL)

func focus_on(world_position: Vector3, zoom_level: float = -1.0) -> void:
	target_position = Vector3(world_position.x, 0, world_position.z)
	if zoom_level > 0:
		target_zoom = clamp(zoom_level, min_zoom, max_zoom)
	_clamp_position()

func _update_camera_transform() -> void:
	if camera == null:
		return
	
	# Calculate base angle from zoom level
	var zoom_t := (current_zoom - min_zoom) / (max_zoom - min_zoom)
	var base_angle := lerp(zoom_angle_near, zoom_angle_far, zoom_t)
	
	# Get target angle for current view mode
	var target_angle: float
	match current_view_mode:
		ViewMode.TACTICAL:
			target_angle = base_angle
		ViewMode.WAR_TABLE:
			target_angle = 89.0  # Nearly top-down
		ViewMode.CINEMATIC:
			target_angle = 25.0  # Low angle, dramatic
		_:
			target_angle = base_angle
	
	# Smooth transition between view modes
	var angle_deg: float
	if view_transition_progress < 1.0:
		# Ease in-out
		var t := _ease_in_out(view_transition_progress)
		angle_deg = lerp(base_angle, target_angle, t)
	else:
		angle_deg = target_angle
	
	var angle_rad := deg_to_rad(angle_deg)
	
	# Position camera relative to pivot
	var height := current_zoom * sin(angle_rad)
	var distance := current_zoom * cos(angle_rad)
	camera.transform.origin = Vector3(0, height, distance)
	
	# Look at pivot point
	camera.look_at(Vector3.ZERO, Vector3.UP)

func _update_camera_transform_immediate() -> void:
	current_zoom = target_zoom
	global_position = target_position
	view_transition_progress = 1.0
	_update_camera_transform()

func _ease_in_out(t: float) -> float:
	return t * t * (3.0 - 2.0 * t)

func get_zoom_band() -> String:
	if current_zoom < 30:
		return "CLOSE"
	elif current_zoom < 80:
		return "MID"
	elif current_zoom < 150:
		return "FAR"
	else:
		return "STRATEGIC"

func get_zoom_normalized() -> float:
	return (current_zoom - min_zoom) / (max_zoom - min_zoom)

## Get the world position under the mouse cursor.
func get_mouse_world_position() -> Vector3:
	if camera == null:
		return Vector3.ZERO
	
	var viewport := get_viewport()
	var mouse_pos := viewport.get_mouse_position()
	var ray_origin := camera.project_ray_origin(mouse_pos)
	var ray_dir := camera.project_ray_normal(mouse_pos)
	
	# Intersect with ground plane (y=0)
	if abs(ray_dir.y) > 0.001:
		var t := -ray_origin.y / ray_dir.y
		if t > 0:
			return ray_origin + ray_dir * t
	return Vector3.ZERO

## Get visible world bounds at ground level.
func get_visible_bounds() -> Rect2:
	if camera == null:
		return Rect2()
	
	var viewport := get_viewport()
	var screen_size := viewport.get_visible_rect().size
	
	# Get corners of visible area
	var corners: Array[Vector3] = []
	for corner in [Vector2.ZERO, Vector2(screen_size.x, 0), screen_size, Vector2(0, screen_size.y)]:
		var ray_origin := camera.project_ray_origin(corner)
		var ray_dir := camera.project_ray_normal(corner)
		if abs(ray_dir.y) > 0.001:
			var t := -ray_origin.y / ray_dir.y
			if t > 0:
				corners.append(ray_origin + ray_dir * t)
	
	if corners.size() < 4:
		return Rect2()
	
	var min_x := corners[0].x
	var max_x := corners[0].x
	var min_z := corners[0].z
	var max_z := corners[0].z
	
	for c in corners:
		min_x = min(min_x, c.x)
		max_x = max(max_x, c.x)
		min_z = min(min_z, c.z)
		max_z = max(max_z, c.z)
	
	return Rect2(min_x, min_z, max_x - min_x, max_z - min_z)
