extends Control
class_name Minimap

## Minimap.gd
## Displays a top-down minimap of the battlefield with unit positions.

signal minimap_clicked(world_position: Vector3)

@export var world_bounds: Rect2 = Rect2(-200, -200, 400, 400)
@export var minimap_size: Vector2 = Vector2(200, 200)
@export var background_color: Color = Color(0.1, 0.15, 0.1, 0.8)
@export var border_color: Color = Color(0.3, 0.4, 0.3)
@export var camera_rect_color: Color = Color(1.0, 1.0, 1.0, 0.5)

var squads: Array[Dictionary] = []
var camera_bounds: Rect2 = Rect2()

func _ready() -> void:
	custom_minimum_size = minimap_size
	size = minimap_size
	mouse_filter = Control.MOUSE_FILTER_STOP

func _draw() -> void:
	# Background
	draw_rect(Rect2(Vector2.ZERO, size), background_color)
	
	# Border
	draw_rect(Rect2(Vector2.ZERO, size), border_color, false, 2.0)
	
	# Grid lines
	_draw_grid()
	
	# Camera view rectangle
	if camera_bounds.size.x > 0:
		var cam_rect := _world_to_minimap_rect(camera_bounds)
		draw_rect(cam_rect, camera_rect_color, false, 1.5)
	
	# Draw units
	for squad in squads:
		_draw_squad(squad)

func _draw_grid() -> void:
	var grid_color := Color(0.2, 0.25, 0.2, 0.5)
	var grid_spacing := 50.0  # World units
	
	# Vertical lines
	var x := world_bounds.position.x
	while x <= world_bounds.end.x:
		var screen_x := _world_x_to_minimap(x)
		draw_line(Vector2(screen_x, 0), Vector2(screen_x, size.y), grid_color, 1.0)
		x += grid_spacing
	
	# Horizontal lines
	var y := world_bounds.position.y
	while y <= world_bounds.end.y:
		var screen_y := _world_z_to_minimap(y)
		draw_line(Vector2(0, screen_y), Vector2(size.x, screen_y), grid_color, 1.0)
		y += grid_spacing

func _draw_squad(squad: Dictionary) -> void:
	var world_pos := Vector2(squad.get("x", 0.0), squad.get("y", 0.0))
	var screen_pos := _world_to_minimap(world_pos)
	
	# Determine color based on faction and state
	var faction: String = squad.get("faction", "Blue")
	var health: float = squad.get("health", 100.0) / squad.get("health_max", 100.0)
	var suppression: float = squad.get("suppression", 0.0)
	var morale: float = squad.get("morale", 1.0)
	
	var base_color: Color
	if faction == "Blue":
		base_color = Color(0.3, 0.5, 1.0)
	else:
		base_color = Color(1.0, 0.3, 0.3)
	
	# Modify color based on state
	if morale < 0.2:
		base_color = base_color.lerp(Color.GRAY, 0.5)
	elif suppression >= 1.0:
		base_color = base_color.lerp(Color.YELLOW, 0.3)
	
	# Size based on health
	var unit_size := 4.0 + 4.0 * health
	
	# Draw unit marker
	draw_circle(screen_pos, unit_size, base_color)
	
	# Draw border
	draw_arc(screen_pos, unit_size, 0, TAU, 16, Color.WHITE, 1.0)
	
	# Draw order indicator
	var order: String = squad.get("order", "Hold")
	if "MoveTo" in order or "AttackMove" in order:
		# Extract target from order string
		var target := _parse_order_target(order)
		if target != Vector2.ZERO:
			var target_screen := _world_to_minimap(target)
			var line_color := Color.GREEN if "MoveTo" in order else Color.RED
			line_color.a = 0.6
			draw_line(screen_pos, target_screen, line_color, 1.0)

func _parse_order_target(order: String) -> Vector2:
	# Parse "MoveTo(x,y)" or "AttackMove(x,y)"
	var start := order.find("(")
	var end := order.find(")")
	if start == -1 or end == -1:
		return Vector2.ZERO
	
	var coords := order.substr(start + 1, end - start - 1)
	var parts := coords.split(",")
	if parts.size() != 2:
		return Vector2.ZERO
	
	return Vector2(float(parts[0]), float(parts[1]))

func _world_to_minimap(world_pos: Vector2) -> Vector2:
	var normalized := (world_pos - world_bounds.position) / world_bounds.size
	return normalized * size

func _world_x_to_minimap(x: float) -> float:
	return (x - world_bounds.position.x) / world_bounds.size.x * size.x

func _world_z_to_minimap(z: float) -> float:
	return (z - world_bounds.position.y) / world_bounds.size.y * size.y

func _world_to_minimap_rect(world_rect: Rect2) -> Rect2:
	var pos := _world_to_minimap(world_rect.position)
	var end := _world_to_minimap(world_rect.end)
	return Rect2(pos, end - pos)

func _minimap_to_world(minimap_pos: Vector2) -> Vector2:
	var normalized := minimap_pos / size
	return world_bounds.position + normalized * world_bounds.size

func update_squads(new_squads: Array) -> void:
	squads.clear()
	for s in new_squads:
		squads.append(s)
	queue_redraw()

func update_camera_bounds(bounds: Rect2) -> void:
	camera_bounds = bounds
	queue_redraw()

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
			var world_pos := _minimap_to_world(event.position)
			emit_signal("minimap_clicked", Vector3(world_pos.x, 0, world_pos.y))
			get_viewport().set_input_as_handled()
