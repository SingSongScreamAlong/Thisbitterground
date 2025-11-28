extends Node3D
class_name TerrainVisualizer

## TerrainVisualizer.gd
## Renders the terrain including ground plane, terrain features, and craters.

signal terrain_loaded()
signal crater_created(position: Vector3, radius: float)

@export var ground_size: float = 400.0
@export var cell_size: float = 2.0
@export var crater_mesh_segments: int = 16

var sim_bridge: SimBridge

# Ground mesh
var ground_mesh: MeshInstance3D

# Terrain type meshes (forest patches, etc.)
var terrain_features: Node3D

# Crater container
var craters_container: Node3D
var crater_meshes: Dictionary = {}  # crater_id -> MeshInstance3D

# Materials
var ground_material: StandardMaterial3D
var road_material: StandardMaterial3D
var forest_material: StandardMaterial3D
var rough_material: StandardMaterial3D
var crater_material: StandardMaterial3D
var mud_material: StandardMaterial3D

# Terrain data cache
var terrain_data: Dictionary = {}
var craters_cache: Array = []

func _ready() -> void:
	_setup_materials()
	_setup_containers()
	_create_ground_plane()

func _setup_materials() -> void:
	# Ground (grass)
	ground_material = StandardMaterial3D.new()
	ground_material.albedo_color = Color(0.25, 0.35, 0.2)
	ground_material.roughness = 0.9
	
	# Road
	road_material = StandardMaterial3D.new()
	road_material.albedo_color = Color(0.4, 0.35, 0.3)
	road_material.roughness = 0.8
	
	# Forest
	forest_material = StandardMaterial3D.new()
	forest_material.albedo_color = Color(0.15, 0.3, 0.1)
	forest_material.roughness = 0.95
	
	# Rough terrain
	rough_material = StandardMaterial3D.new()
	rough_material.albedo_color = Color(0.35, 0.3, 0.25)
	rough_material.roughness = 0.95
	
	# Crater
	crater_material = StandardMaterial3D.new()
	crater_material.albedo_color = Color(0.2, 0.18, 0.15)
	crater_material.roughness = 1.0
	
	# Mud
	mud_material = StandardMaterial3D.new()
	mud_material.albedo_color = Color(0.3, 0.25, 0.2)
	mud_material.roughness = 0.7

func _setup_containers() -> void:
	terrain_features = Node3D.new()
	terrain_features.name = "TerrainFeatures"
	add_child(terrain_features)
	
	craters_container = Node3D.new()
	craters_container.name = "Craters"
	add_child(craters_container)

func _create_ground_plane() -> void:
	ground_mesh = MeshInstance3D.new()
	ground_mesh.name = "Ground"
	
	var plane := PlaneMesh.new()
	plane.size = Vector2(ground_size, ground_size)
	plane.subdivide_width = 4
	plane.subdivide_depth = 4
	ground_mesh.mesh = plane
	ground_mesh.set_surface_override_material(0, ground_material)
	
	add_child(ground_mesh)

func set_sim_bridge(bridge: SimBridge) -> void:
	sim_bridge = bridge
	# Load initial terrain
	call_deferred("_load_terrain")

func _load_terrain() -> void:
	if not sim_bridge:
		return
	
	terrain_data = sim_bridge.get_terrain()
	if terrain_data.is_empty():
		print("[TerrainVisualizer] No terrain data available")
		return
	
	_create_terrain_features()
	_load_craters()
	emit_signal("terrain_loaded")

func _create_terrain_features() -> void:
	# Clear existing features
	for child in terrain_features.get_children():
		child.queue_free()
	
	if terrain_data.is_empty():
		return
	
	var width: int = terrain_data.get("width", 0)
	var height: int = terrain_data.get("height", 0)
	var types: Array = terrain_data.get("types", [])
	var origin_x: float = terrain_data.get("origin_x", -ground_size / 2)
	var origin_y: float = terrain_data.get("origin_y", -ground_size / 2)
	var cell_sz: float = terrain_data.get("cell_size", cell_size)
	
	if types.is_empty():
		return
	
	# Create feature meshes for non-open terrain
	# Group adjacent cells of same type for efficiency
	var processed := {}
	
	for y in range(height):
		for x in range(width):
			var idx := y * width + x
			if idx >= types.size():
				continue
			
			var terrain_type: int = types[idx]
			if terrain_type == 0:  # Open terrain, skip
				continue
			
			var key := "%d_%d" % [x, y]
			if processed.has(key):
				continue
			
			processed[key] = true
			
			# Create mesh for this cell
			var world_x := origin_x + (x + 0.5) * cell_sz
			var world_z := origin_y + (y + 0.5) * cell_sz
			
			_create_terrain_cell_mesh(terrain_type, world_x, world_z, cell_sz)

func _create_terrain_cell_mesh(terrain_type: int, world_x: float, world_z: float, size: float) -> void:
	var mesh_instance := MeshInstance3D.new()
	
	match terrain_type:
		1:  # Rough
			var box := BoxMesh.new()
			box.size = Vector3(size, 0.2, size)
			mesh_instance.mesh = box
			mesh_instance.set_surface_override_material(0, rough_material)
			mesh_instance.position = Vector3(world_x, 0.1, world_z)
		2:  # Mud
			var box := BoxMesh.new()
			box.size = Vector3(size, 0.1, size)
			mesh_instance.mesh = box
			mesh_instance.set_surface_override_material(0, mud_material)
			mesh_instance.position = Vector3(world_x, 0.05, world_z)
		3:  # Crater - handled separately
			return
		4:  # Trench
			var box := BoxMesh.new()
			box.size = Vector3(size, 0.5, size)
			mesh_instance.mesh = box
			mesh_instance.set_surface_override_material(0, crater_material)
			mesh_instance.position = Vector3(world_x, -0.25, world_z)
		5:  # Water
			var box := BoxMesh.new()
			box.size = Vector3(size, 0.1, size)
			mesh_instance.mesh = box
			var water_mat := StandardMaterial3D.new()
			water_mat.albedo_color = Color(0.2, 0.3, 0.5, 0.7)
			water_mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
			mesh_instance.set_surface_override_material(0, water_mat)
			mesh_instance.position = Vector3(world_x, 0.05, world_z)
		6:  # Road
			var box := BoxMesh.new()
			box.size = Vector3(size, 0.05, size)
			mesh_instance.mesh = box
			mesh_instance.set_surface_override_material(0, road_material)
			mesh_instance.position = Vector3(world_x, 0.025, world_z)
		7:  # Forest
			_create_forest_patch(world_x, world_z, size)
			return
		8:  # Rubble
			var box := BoxMesh.new()
			box.size = Vector3(size, 0.3, size)
			mesh_instance.mesh = box
			mesh_instance.set_surface_override_material(0, rough_material)
			mesh_instance.position = Vector3(world_x, 0.15, world_z)
		_:
			return
	
	terrain_features.add_child(mesh_instance)

func _create_forest_patch(world_x: float, world_z: float, size: float) -> void:
	# Create a few simple tree representations
	var tree_count := randi_range(2, 4)
	
	for i in range(tree_count):
		var tree := MeshInstance3D.new()
		var cone := CylinderMesh.new()
		cone.top_radius = 0.0
		cone.bottom_radius = size * 0.3
		cone.height = size * 1.5
		tree.mesh = cone
		tree.set_surface_override_material(0, forest_material)
		
		var offset_x := randf_range(-size * 0.3, size * 0.3)
		var offset_z := randf_range(-size * 0.3, size * 0.3)
		tree.position = Vector3(world_x + offset_x, size * 0.75, world_z + offset_z)
		
		terrain_features.add_child(tree)
		
		# Add trunk
		var trunk := MeshInstance3D.new()
		var trunk_cyl := CylinderMesh.new()
		trunk_cyl.top_radius = size * 0.05
		trunk_cyl.bottom_radius = size * 0.08
		trunk_cyl.height = size * 0.5
		trunk.mesh = trunk_cyl
		
		var trunk_mat := StandardMaterial3D.new()
		trunk_mat.albedo_color = Color(0.3, 0.2, 0.1)
		trunk.set_surface_override_material(0, trunk_mat)
		trunk.position = Vector3(world_x + offset_x, size * 0.25, world_z + offset_z)
		
		terrain_features.add_child(trunk)

func _load_craters() -> void:
	if not sim_bridge:
		return
	
	craters_cache = sim_bridge.get_craters()
	
	for crater in craters_cache:
		_create_crater_mesh(crater)

func update_craters(new_craters: Array) -> void:
	for crater in new_craters:
		if not _crater_exists(crater):
			_create_crater_mesh(crater)
			craters_cache.append(crater)
			emit_signal("crater_created", Vector3(crater["x"], 0, crater["y"]), crater["radius"])

func _crater_exists(crater: Dictionary) -> bool:
	var key := "%.1f_%.1f" % [crater["x"], crater["y"]]
	return crater_meshes.has(key)

func _create_crater_mesh(crater: Dictionary) -> void:
	var x: float = crater.get("x", 0.0)
	var y: float = crater.get("y", 0.0)
	var radius: float = crater.get("radius", 3.0)
	var depth: float = crater.get("depth", 1.0)
	
	var key := "%.1f_%.1f" % [x, y]
	
	# Create crater depression mesh
	var crater_mesh := MeshInstance3D.new()
	
	# Use a cylinder for the crater
	var cyl := CylinderMesh.new()
	cyl.top_radius = radius
	cyl.bottom_radius = radius * 0.6
	cyl.height = depth
	cyl.radial_segments = crater_mesh_segments
	crater_mesh.mesh = cyl
	crater_mesh.set_surface_override_material(0, crater_material)
	
	# Position crater (inverted, going into ground)
	crater_mesh.position = Vector3(x, -depth / 2, y)
	
	craters_container.add_child(crater_mesh)
	crater_meshes[key] = crater_mesh
	
	# Create rim around crater
	var rim := MeshInstance3D.new()
	var rim_torus := TorusMesh.new()
	rim_torus.inner_radius = radius * 0.9
	rim_torus.outer_radius = radius * 1.2
	rim.mesh = rim_torus
	rim.rotation.x = -PI / 2
	rim.position = Vector3(x, 0.1, y)
	
	var rim_mat := StandardMaterial3D.new()
	rim_mat.albedo_color = Color(0.25, 0.22, 0.18)
	rim.set_surface_override_material(0, rim_mat)
	
	craters_container.add_child(rim)

func get_terrain_type_at(world_x: float, world_z: float) -> int:
	if terrain_data.is_empty():
		return 0
	
	var width: int = terrain_data.get("width", 0)
	var height: int = terrain_data.get("height", 0)
	var types: Array = terrain_data.get("types", [])
	var origin_x: float = terrain_data.get("origin_x", -ground_size / 2)
	var origin_y: float = terrain_data.get("origin_y", -ground_size / 2)
	var cell_sz: float = terrain_data.get("cell_size", cell_size)
	
	var gx := int((world_x - origin_x) / cell_sz)
	var gy := int((world_z - origin_y) / cell_sz)
	
	gx = clampi(gx, 0, width - 1)
	gy = clampi(gy, 0, height - 1)
	
	var idx := gy * width + gx
	if idx < types.size():
		return types[idx]
	return 0

func clear_craters() -> void:
	for child in craters_container.get_children():
		child.queue_free()
	crater_meshes.clear()
	craters_cache.clear()
