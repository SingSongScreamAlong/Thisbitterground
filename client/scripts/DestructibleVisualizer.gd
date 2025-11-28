extends Node3D
class_name DestructibleVisualizer

## DestructibleVisualizer.gd
## Visualizes a single destructible object (tree, building, etc.)

signal destroyed(id: int)
signal damaged(id: int)

@export var destructible_id: int = 0
@export var destructible_type: String = "Tree"

var current_state: String = "Intact"
var health_fraction: float = 1.0

# Visual components
var mesh_intact: MeshInstance3D
var mesh_damaged: MeshInstance3D
var mesh_destroyed: MeshInstance3D

# Materials
var tree_material: StandardMaterial3D
var tree_damaged_material: StandardMaterial3D
var building_material: StandardMaterial3D
var building_damaged_material: StandardMaterial3D
var rubble_material: StandardMaterial3D

func _ready() -> void:
	_setup_materials()
	_create_visuals()

func _setup_materials() -> void:
	# Tree materials
	tree_material = StandardMaterial3D.new()
	tree_material.albedo_color = Color(0.15, 0.4, 0.1)
	tree_material.roughness = 0.9
	
	tree_damaged_material = StandardMaterial3D.new()
	tree_damaged_material.albedo_color = Color(0.3, 0.25, 0.1)
	tree_damaged_material.roughness = 0.95
	
	# Building materials
	building_material = StandardMaterial3D.new()
	building_material.albedo_color = Color(0.5, 0.45, 0.4)
	building_material.roughness = 0.8
	
	building_damaged_material = StandardMaterial3D.new()
	building_damaged_material.albedo_color = Color(0.35, 0.3, 0.25)
	building_damaged_material.roughness = 0.9
	
	# Rubble material
	rubble_material = StandardMaterial3D.new()
	rubble_material.albedo_color = Color(0.3, 0.28, 0.25)
	rubble_material.roughness = 1.0

func _create_visuals() -> void:
	match destructible_type:
		"Tree":
			_create_tree_visuals()
		"Building":
			_create_building_visuals()
		_:
			_create_tree_visuals()  # Default to tree

func _create_tree_visuals() -> void:
	# Intact tree - cone shape
	mesh_intact = MeshInstance3D.new()
	var cone := CylinderMesh.new()
	cone.top_radius = 0.0
	cone.bottom_radius = 1.5
	cone.height = 4.0
	mesh_intact.mesh = cone
	mesh_intact.set_surface_override_material(0, tree_material)
	mesh_intact.position.y = 2.0
	add_child(mesh_intact)
	
	# Trunk
	var trunk := MeshInstance3D.new()
	var trunk_mesh := CylinderMesh.new()
	trunk_mesh.top_radius = 0.2
	trunk_mesh.bottom_radius = 0.3
	trunk_mesh.height = 1.5
	trunk.mesh = trunk_mesh
	var trunk_mat := StandardMaterial3D.new()
	trunk_mat.albedo_color = Color(0.35, 0.25, 0.15)
	trunk.set_surface_override_material(0, trunk_mat)
	trunk.position.y = 0.75
	mesh_intact.add_child(trunk)
	
	# Damaged tree - smaller, tilted
	mesh_damaged = MeshInstance3D.new()
	var damaged_cone := CylinderMesh.new()
	damaged_cone.top_radius = 0.0
	damaged_cone.bottom_radius = 1.0
	damaged_cone.height = 2.5
	mesh_damaged.mesh = damaged_cone
	mesh_damaged.set_surface_override_material(0, tree_damaged_material)
	mesh_damaged.position.y = 1.25
	mesh_damaged.rotation.z = 0.3
	mesh_damaged.visible = false
	add_child(mesh_damaged)
	
	# Destroyed - stump
	mesh_destroyed = MeshInstance3D.new()
	var stump := CylinderMesh.new()
	stump.top_radius = 0.25
	stump.bottom_radius = 0.35
	stump.height = 0.5
	mesh_destroyed.mesh = stump
	mesh_destroyed.set_surface_override_material(0, rubble_material)
	mesh_destroyed.position.y = 0.25
	mesh_destroyed.visible = false
	add_child(mesh_destroyed)

func _create_building_visuals() -> void:
	# Intact building - box
	mesh_intact = MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = Vector3(6.0, 4.0, 6.0)
	mesh_intact.mesh = box
	mesh_intact.set_surface_override_material(0, building_material)
	mesh_intact.position.y = 2.0
	add_child(mesh_intact)
	
	# Roof
	var roof := MeshInstance3D.new()
	var roof_mesh := PrismMesh.new()
	roof_mesh.size = Vector3(7.0, 2.0, 7.0)
	roof.mesh = roof_mesh
	var roof_mat := StandardMaterial3D.new()
	roof_mat.albedo_color = Color(0.4, 0.2, 0.15)
	roof.set_surface_override_material(0, roof_mat)
	roof.position.y = 3.0
	mesh_intact.add_child(roof)
	
	# Damaged building - partial walls
	mesh_damaged = MeshInstance3D.new()
	var damaged_box := BoxMesh.new()
	damaged_box.size = Vector3(6.0, 2.5, 6.0)
	mesh_damaged.mesh = damaged_box
	mesh_damaged.set_surface_override_material(0, building_damaged_material)
	mesh_damaged.position.y = 1.25
	mesh_damaged.visible = false
	add_child(mesh_damaged)
	
	# Destroyed - rubble pile
	mesh_destroyed = MeshInstance3D.new()
	var rubble := BoxMesh.new()
	rubble.size = Vector3(7.0, 1.0, 7.0)
	mesh_destroyed.mesh = rubble
	mesh_destroyed.set_surface_override_material(0, rubble_material)
	mesh_destroyed.position.y = 0.5
	mesh_destroyed.visible = false
	add_child(mesh_destroyed)

func update_state(new_state: String, health: float, health_max: float) -> void:
	health_fraction = health / health_max if health_max > 0 else 0.0
	
	if new_state != current_state:
		var old_state := current_state
		current_state = new_state
		_update_visuals()
		
		if new_state == "Destroyed" and old_state != "Destroyed":
			emit_signal("destroyed", destructible_id)
		elif new_state == "Damaged" and old_state == "Intact":
			emit_signal("damaged", destructible_id)

func _update_visuals() -> void:
	if mesh_intact:
		mesh_intact.visible = (current_state == "Intact")
	if mesh_damaged:
		mesh_damaged.visible = (current_state == "Damaged")
	if mesh_destroyed:
		mesh_destroyed.visible = (current_state == "Destroyed")

func set_type(dtype: String) -> void:
	destructible_type = dtype
	# Clear existing visuals
	for child in get_children():
		child.queue_free()
	mesh_intact = null
	mesh_damaged = null
	mesh_destroyed = null
	# Recreate
	call_deferred("_create_visuals")
