extends Node3D
class_name UnitVisualizer

## UnitVisualizer.gd
## Handles visualization of a single squad/unit.
## Supports multiple LOD levels based on camera distance.
## - CLOSE: Individual soldier meshes
## - MID: Squad box with details
## - FAR: Simplified icon
## - STRATEGIC: NATO symbol

signal clicked(squad_id: int)
signal hovered(squad_id: int, is_hovered: bool)

@export var squad_id: int = -1
@export var faction: String = "Blue"
@export var squad_size: int = 12

# LOD containers
var lod_close: Node3D      # Individual soldiers
var lod_mid: Node3D        # Squad box
var lod_far: Node3D        # Icon sprite
var lod_strategic: Node3D  # NATO symbol

var selection_ring: MeshInstance3D
var health_bar: MeshInstance3D
var suppression_indicator: MeshInstance3D
var order_line: MeshInstance3D

var is_selected: bool = false
var is_hovered: bool = false
var current_zoom_band: String = "MID"

# Visual state
var health_fraction: float = 1.0
var suppression: float = 0.0
var morale: float = 1.0
var current_order: String = "Hold"
var order_target: Vector3 = Vector3.ZERO

# Animation
var suppression_pulse_time: float = 0.0
var is_routing: bool = false

func _ready() -> void:
	_setup_visuals()

func _process(delta: float) -> void:
	_update_animations(delta)

func _setup_visuals() -> void:
	# LOD Close: Individual soldiers (simplified as small boxes)
	lod_close = Node3D.new()
	lod_close.name = "LOD_Close"
	add_child(lod_close)
	_create_soldier_meshes()
	
	# LOD Mid: Squad box
	lod_mid = Node3D.new()
	lod_mid.name = "LOD_Mid"
	add_child(lod_mid)
	_create_squad_box()
	
	# LOD Far: Icon sprite
	lod_far = Node3D.new()
	lod_far.name = "LOD_Far"
	lod_far.visible = false
	add_child(lod_far)
	_create_icon_sprite()
	
	# LOD Strategic: NATO symbol
	lod_strategic = Node3D.new()
	lod_strategic.name = "LOD_Strategic"
	lod_strategic.visible = false
	add_child(lod_strategic)
	_create_nato_symbol()
	
	# Selection ring
	selection_ring = MeshInstance3D.new()
	var torus := TorusMesh.new()
	torus.inner_radius = 3.5
	torus.outer_radius = 4.0
	selection_ring.mesh = torus
	selection_ring.rotation.x = -PI / 2
	selection_ring.visible = false
	add_child(selection_ring)
	
	# Health bar (floating above unit)
	health_bar = MeshInstance3D.new()
	var health_box := BoxMesh.new()
	health_box.size = Vector3(4, 0.3, 0.1)
	health_bar.mesh = health_box
	health_bar.position = Vector3(0, 4, 0)
	add_child(health_bar)
	
	# Suppression indicator (pulsing ring)
	suppression_indicator = MeshInstance3D.new()
	var sup_torus := TorusMesh.new()
	sup_torus.inner_radius = 2.8
	sup_torus.outer_radius = 3.2
	suppression_indicator.mesh = sup_torus
	suppression_indicator.rotation.x = -PI / 2
	suppression_indicator.visible = false
	add_child(suppression_indicator)
	
	# Order line (shows movement target)
	order_line = MeshInstance3D.new()
	order_line.visible = false
	add_child(order_line)
	
	_update_materials()

func _create_soldier_meshes() -> void:
	# Create a grid of small soldier representations
	var rows := 3
	var cols := 4
	var spacing := 1.2
	
	for row in range(rows):
		for col in range(cols):
			var soldier := MeshInstance3D.new()
			var capsule := CapsuleMesh.new()
			capsule.radius = 0.3
			capsule.height = 1.2
			soldier.mesh = capsule
			
			var x := (col - (cols - 1) / 2.0) * spacing
			var z := (row - (rows - 1) / 2.0) * spacing
			soldier.position = Vector3(x, 0.6, z)
			
			lod_close.add_child(soldier)

func _create_squad_box() -> void:
	var mesh := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = Vector3(4, 1.5, 3)
	mesh.mesh = box
	mesh.position.y = 0.75
	lod_mid.add_child(mesh)
	
	# Add faction banner
	var banner := MeshInstance3D.new()
	var banner_box := BoxMesh.new()
	banner_box.size = Vector3(0.2, 2, 0.2)
	banner.mesh = banner_box
	banner.position = Vector3(-1.5, 1.5, -1)
	lod_mid.add_child(banner)

func _create_icon_sprite() -> void:
	var sprite := Sprite3D.new()
	sprite.pixel_size = 0.15
	sprite.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	sprite.position.y = 2.0
	
	# Create a simple colored quad texture
	var img := Image.create(32, 32, false, Image.FORMAT_RGBA8)
	var color := Color(0.3, 0.5, 0.9) if faction == "Blue" else Color(0.9, 0.3, 0.3)
	img.fill(color)
	# Add border
	for i in range(32):
		img.set_pixel(i, 0, Color.BLACK)
		img.set_pixel(i, 31, Color.BLACK)
		img.set_pixel(0, i, Color.BLACK)
		img.set_pixel(31, i, Color.BLACK)
	
	var tex := ImageTexture.create_from_image(img)
	sprite.texture = tex
	lod_far.add_child(sprite)

func _create_nato_symbol() -> void:
	# Create NATO infantry symbol (rectangle with X)
	var base := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = Vector3(6, 0.2, 4)
	base.mesh = box
	base.position.y = 0.1
	lod_strategic.add_child(base)
	
	# Add X lines
	var line1 := MeshInstance3D.new()
	var line_box := BoxMesh.new()
	line_box.size = Vector3(7, 0.3, 0.3)
	line1.mesh = line_box
	line1.rotation.y = PI / 4
	line1.position.y = 0.3
	lod_strategic.add_child(line1)
	
	var line2 := MeshInstance3D.new()
	line2.mesh = line_box
	line2.rotation.y = -PI / 4
	line2.position.y = 0.3
	lod_strategic.add_child(line2)

func set_faction(new_faction: String) -> void:
	faction = new_faction
	_update_materials()

func set_selected(selected: bool) -> void:
	is_selected = selected
	selection_ring.visible = selected
	_update_order_line()

func set_hovered(hovered: bool) -> void:
	if is_hovered != hovered:
		is_hovered = hovered
		emit_signal("hovered", squad_id, is_hovered)
		_update_hover_effect()

func set_zoom_band(band: String) -> void:
	if current_zoom_band != band:
		current_zoom_band = band
		_update_lod()

func update_state(health_frac: float, sup: float, mor: float, order: String = "Hold", target: Vector3 = Vector3.ZERO) -> void:
	health_fraction = health_frac
	suppression = sup
	morale = mor
	current_order = order
	order_target = target
	is_routing = morale < 0.2
	_update_visual_state()
	_update_order_line()

func _update_materials() -> void:
	var base_color := Color(0.2, 0.4, 0.8) if faction == "Blue" else Color(0.8, 0.2, 0.2)
	var dark_color := base_color.darkened(0.3)
	
	# Soldier material
	var soldier_mat := StandardMaterial3D.new()
	soldier_mat.albedo_color = base_color
	soldier_mat.roughness = 0.8
	for child in lod_close.get_children():
		if child is MeshInstance3D:
			child.set_surface_override_material(0, soldier_mat)
	
	# Squad box material
	var box_mat := StandardMaterial3D.new()
	box_mat.albedo_color = base_color
	box_mat.roughness = 0.6
	for child in lod_mid.get_children():
		if child is MeshInstance3D:
			child.set_surface_override_material(0, box_mat)
	
	# NATO symbol material
	var nato_mat := StandardMaterial3D.new()
	nato_mat.albedo_color = dark_color
	for child in lod_strategic.get_children():
		if child is MeshInstance3D:
			child.set_surface_override_material(0, nato_mat)
	
	# Selection ring material
	var ring_mat := StandardMaterial3D.new()
	ring_mat.albedo_color = Color(1.0, 1.0, 0.3, 0.8)
	ring_mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	ring_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	selection_ring.set_surface_override_material(0, ring_mat)
	
	# Health bar material
	var health_mat := StandardMaterial3D.new()
	health_mat.albedo_color = Color(0.2, 0.8, 0.2)
	health_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	health_bar.set_surface_override_material(0, health_mat)
	
	# Suppression indicator material
	var sup_mat := StandardMaterial3D.new()
	sup_mat.albedo_color = Color(1.0, 0.8, 0.0, 0.6)
	sup_mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	sup_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	suppression_indicator.set_surface_override_material(0, sup_mat)

func _update_lod() -> void:
	lod_close.visible = false
	lod_mid.visible = false
	lod_far.visible = false
	lod_strategic.visible = false
	
	match current_zoom_band:
		"CLOSE":
			lod_close.visible = true
			health_bar.visible = true
		"MID":
			lod_mid.visible = true
			health_bar.visible = true
		"FAR":
			lod_far.visible = true
			health_bar.visible = false
		"STRATEGIC":
			lod_strategic.visible = true
			health_bar.visible = false

func _update_visual_state() -> void:
	# Update health bar
	var health_color: Color
	if health_fraction > 0.6:
		health_color = Color(0.2, 0.8, 0.2)
	elif health_fraction > 0.3:
		health_color = Color(0.9, 0.7, 0.1)
	else:
		health_color = Color(0.9, 0.2, 0.2)
	
	var health_mat: StandardMaterial3D = health_bar.get_surface_override_material(0)
	if health_mat:
		health_mat.albedo_color = health_color
	health_bar.scale.x = health_fraction
	health_bar.position.x = -2.0 * (1.0 - health_fraction)
	
	# Update suppression indicator
	suppression_indicator.visible = suppression >= 0.3
	
	# Update soldier visibility based on casualties
	var visible_soldiers := int(squad_size * health_fraction)
	var idx := 0
	for child in lod_close.get_children():
		if child is MeshInstance3D:
			child.visible = idx < visible_soldiers
			idx += 1
	
	# Tint based on morale
	if is_routing:
		_apply_routing_tint()

func _update_hover_effect() -> void:
	var scale_target := 1.1 if is_hovered else 1.0
	var tween := create_tween()
	tween.tween_property(self, "scale", Vector3.ONE * scale_target, 0.1)

func _update_order_line() -> void:
	if not is_selected or current_order == "Hold":
		order_line.visible = false
		return
	
	# Create line to target
	var start := Vector3.ZERO
	var end := order_target - global_position
	end.y = 0.5
	
	var length := end.length()
	if length < 1.0:
		order_line.visible = false
		return
	
	order_line.visible = true
	
	# Create cylinder mesh for line
	var cyl := CylinderMesh.new()
	cyl.top_radius = 0.15
	cyl.bottom_radius = 0.15
	cyl.height = length
	order_line.mesh = cyl
	
	# Position and rotate
	order_line.position = end / 2.0
	order_line.position.y = 0.5
	order_line.look_at(global_position + end, Vector3.UP)
	order_line.rotate_object_local(Vector3.RIGHT, PI / 2)
	
	# Color based on order type
	var line_mat := StandardMaterial3D.new()
	if "Attack" in current_order:
		line_mat.albedo_color = Color(1.0, 0.3, 0.3, 0.7)
	else:
		line_mat.albedo_color = Color(0.3, 1.0, 0.3, 0.7)
	line_mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	line_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	order_line.set_surface_override_material(0, line_mat)

func _update_animations(delta: float) -> void:
	# Suppression pulse
	if suppression >= 0.3:
		suppression_pulse_time += delta * 3.0
		var pulse := 0.8 + 0.2 * sin(suppression_pulse_time * PI)
		suppression_indicator.scale = Vector3(pulse, 1.0, pulse)
		
		var sup_mat: StandardMaterial3D = suppression_indicator.get_surface_override_material(0)
		if sup_mat:
			sup_mat.albedo_color.a = 0.3 + 0.3 * suppression

func _apply_routing_tint() -> void:
	var grey := Color(0.5, 0.5, 0.5)
	for child in lod_close.get_children():
		if child is MeshInstance3D:
			var mat: StandardMaterial3D = child.get_surface_override_material(0)
			if mat:
				mat.albedo_color = mat.albedo_color.lerp(grey, 0.5)

func _input_event(_camera: Node, event: InputEvent, _position: Vector3, _normal: Vector3, _shape_idx: int) -> void:
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
		emit_signal("clicked", squad_id)
