extends Node
class_name SelectionManager

## SelectionManager.gd
## Handles unit selection and order input.

signal selection_changed(selected_ids: Array[int])
signal order_issued(order_type: String, squad_ids: Array[int], target: Vector3)

@export var camera_controller: CameraController
@export var sim_bridge: SimBridge

# Selection state
var selected_squad_ids: Array[int] = []
var hovered_squad_id: int = -1

# Box selection
var is_box_selecting: bool = false
var box_start: Vector2 = Vector2.ZERO
var box_end: Vector2 = Vector2.ZERO

# Squad visualizers (set by Battlefield)
var squad_visualizers: Dictionary = {}  # squad_id -> UnitVisualizer

func _ready() -> void:
	set_process_input(true)

func _unhandled_input(event: InputEvent) -> void:
	# Left click - select or start box select
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		if event.pressed:
			_start_selection(event.position)
		else:
			_end_selection(event.position)
	
	# Right click - issue order
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_RIGHT and event.pressed:
		_issue_order(event.position, event.shift_pressed)
	
	# Box selection drag
	if event is InputEventMouseMotion and is_box_selecting:
		box_end = event.position
	
	# Keyboard shortcuts
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_ESCAPE:
				clear_selection()
			KEY_H:
				_issue_hold_order()
			KEY_A:
				if selected_squad_ids.size() > 0:
					# Enter attack-move mode
					pass

func _start_selection(mouse_pos: Vector2) -> void:
	var clicked_id := _get_squad_at_screen_pos(mouse_pos)
	
	if clicked_id >= 0:
		# Clicked on a unit
		if Input.is_key_pressed(KEY_SHIFT):
			# Add to selection
			if clicked_id not in selected_squad_ids:
				selected_squad_ids.append(clicked_id)
		elif Input.is_key_pressed(KEY_CTRL):
			# Toggle selection
			if clicked_id in selected_squad_ids:
				selected_squad_ids.erase(clicked_id)
			else:
				selected_squad_ids.append(clicked_id)
		else:
			# Replace selection
			selected_squad_ids = [clicked_id]
		_update_selection_visuals()
	else:
		# Start box selection
		is_box_selecting = true
		box_start = mouse_pos
		box_end = mouse_pos
		if not Input.is_key_pressed(KEY_SHIFT):
			selected_squad_ids.clear()

func _end_selection(mouse_pos: Vector2) -> void:
	if is_box_selecting:
		is_box_selecting = false
		box_end = mouse_pos
		_process_box_selection()
	_update_selection_visuals()

func _process_box_selection() -> void:
	var rect := Rect2(box_start, box_end - box_start).abs()
	
	# Only process if box is large enough
	if rect.size.x < 5 or rect.size.y < 5:
		return
	
	# Find all squads in the box
	for squad_id in squad_visualizers:
		var visualizer: UnitVisualizer = squad_visualizers[squad_id]
		var screen_pos := _world_to_screen(visualizer.global_position)
		
		if rect.has_point(screen_pos):
			# Only select player's faction (Blue)
			if visualizer.faction == "Blue" and squad_id not in selected_squad_ids:
				selected_squad_ids.append(squad_id)

func _issue_order(mouse_pos: Vector2, attack_move: bool) -> void:
	if selected_squad_ids.is_empty():
		return
	
	var world_pos := _get_world_position(mouse_pos)
	if world_pos == Vector3.ZERO:
		return
	
	var order_type := "AttackMove" if attack_move else "MoveTo"
	
	# Issue orders through SimBridge
	if sim_bridge:
		for squad_id in selected_squad_ids:
			if attack_move:
				sim_bridge.order_attack_move(squad_id, world_pos.x, world_pos.z)
			else:
				sim_bridge.order_move(squad_id, world_pos.x, world_pos.z)
	
	emit_signal("order_issued", order_type, selected_squad_ids.duplicate(), world_pos)
	print("[Selection] %s order to (%.1f, %.1f) for %d squads" % [order_type, world_pos.x, world_pos.z, selected_squad_ids.size()])

func _issue_hold_order() -> void:
	if selected_squad_ids.is_empty():
		return
	
	if sim_bridge:
		for squad_id in selected_squad_ids:
			sim_bridge.order_hold(squad_id)
	
	emit_signal("order_issued", "Hold", selected_squad_ids.duplicate(), Vector3.ZERO)
	print("[Selection] Hold order for %d squads" % selected_squad_ids.size())

func _get_squad_at_screen_pos(screen_pos: Vector2) -> int:
	# Raycast to find squad under mouse
	if camera_controller == null or camera_controller.camera == null:
		return -1
	
	var world_pos := _get_world_position(screen_pos)
	if world_pos == Vector3.ZERO:
		return -1
	
	# Find closest squad to world position
	var closest_id := -1
	var closest_dist := 5.0  # Max click distance
	
	for squad_id in squad_visualizers:
		var visualizer: UnitVisualizer = squad_visualizers[squad_id]
		var dist := visualizer.global_position.distance_to(world_pos)
		if dist < closest_dist:
			closest_dist = dist
			closest_id = squad_id
	
	return closest_id

func _get_world_position(screen_pos: Vector2) -> Vector3:
	if camera_controller:
		# Use camera controller's method
		var viewport := get_viewport()
		var camera := camera_controller.camera
		if camera:
			var ray_origin := camera.project_ray_origin(screen_pos)
			var ray_dir := camera.project_ray_normal(screen_pos)
			
			# Intersect with ground plane
			if abs(ray_dir.y) > 0.001:
				var t := -ray_origin.y / ray_dir.y
				if t > 0:
					return ray_origin + ray_dir * t
	return Vector3.ZERO

func _world_to_screen(world_pos: Vector3) -> Vector2:
	if camera_controller and camera_controller.camera:
		return camera_controller.camera.unproject_position(world_pos)
	return Vector2.ZERO

func _update_selection_visuals() -> void:
	for squad_id in squad_visualizers:
		var visualizer: UnitVisualizer = squad_visualizers[squad_id]
		visualizer.set_selected(squad_id in selected_squad_ids)
	
	emit_signal("selection_changed", selected_squad_ids.duplicate())

func clear_selection() -> void:
	selected_squad_ids.clear()
	_update_selection_visuals()

func select_all_friendly() -> void:
	selected_squad_ids.clear()
	for squad_id in squad_visualizers:
		var visualizer: UnitVisualizer = squad_visualizers[squad_id]
		if visualizer.faction == "Blue":
			selected_squad_ids.append(squad_id)
	_update_selection_visuals()

func register_visualizer(squad_id: int, visualizer: UnitVisualizer) -> void:
	squad_visualizers[squad_id] = visualizer
	visualizer.clicked.connect(_on_visualizer_clicked)

func unregister_visualizer(squad_id: int) -> void:
	if squad_id in squad_visualizers:
		var visualizer: UnitVisualizer = squad_visualizers[squad_id]
		if visualizer.clicked.is_connected(_on_visualizer_clicked):
			visualizer.clicked.disconnect(_on_visualizer_clicked)
		squad_visualizers.erase(squad_id)

func _on_visualizer_clicked(squad_id: int) -> void:
	if Input.is_key_pressed(KEY_SHIFT):
		if squad_id not in selected_squad_ids:
			selected_squad_ids.append(squad_id)
	elif Input.is_key_pressed(KEY_CTRL):
		if squad_id in selected_squad_ids:
			selected_squad_ids.erase(squad_id)
		else:
			selected_squad_ids.append(squad_id)
	else:
		selected_squad_ids = [squad_id]
	_update_selection_visuals()

func get_selection_box() -> Rect2:
	if is_box_selecting:
		return Rect2(box_start, box_end - box_start).abs()
	return Rect2()
