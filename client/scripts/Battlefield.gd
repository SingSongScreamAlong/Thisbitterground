extends Node3D

## Battlefield.gd
## Manages the visual representation of the battlefield.
## Receives snapshots from SimBridge and updates squad visuals.
## Integrates with SelectionManager, CameraController, and TerrainVisualizer.

@onready var squads_container: Node3D = $Squads
@onready var terrain_visualizer: TerrainVisualizer = $TerrainVisualizer

var sim_bridge: SimBridge
var camera_controller: CameraController
var selection_manager: SelectionManager
var squad_visuals: Dictionary = {}  # squad_id -> UnitVisualizer
var destructible_visuals: Dictionary = {}  # destructible_id -> DestructibleVisualizer

# Destructibles container
var destructibles_container: Node3D

# Current zoom band for LOD
var current_zoom_band: String = "MID"

func _ready() -> void:
	# Create destructibles container
	destructibles_container = Node3D.new()
	destructibles_container.name = "Destructibles"
	add_child(destructibles_container)

func set_sim_bridge(bridge: SimBridge) -> void:
	sim_bridge = bridge
	if terrain_visualizer:
		terrain_visualizer.set_sim_bridge(bridge)

func set_camera_controller(controller: CameraController) -> void:
	camera_controller = controller
	if camera_controller:
		camera_controller.zoom_changed.connect(_on_zoom_changed)
		_update_zoom_band()

func set_selection_manager(manager: SelectionManager) -> void:
	selection_manager = manager

func update_from_snapshot(snapshot: Dictionary) -> void:
	var squad_list: Array = snapshot.get("squads", [])
	var active_ids: Array[int] = []
	
	for squad_data in squad_list:
		var squad_id: int = squad_data["id"]
		active_ids.append(squad_id)
		
		# Create or update visual
		if not squad_visuals.has(squad_id):
			_create_squad_visual(squad_id, squad_data)
		
		_update_squad_visual(squad_id, squad_data)
	
	# Remove visuals for squads that no longer exist
	var to_remove: Array[int] = []
	for id in squad_visuals:
		if id not in active_ids:
			to_remove.append(id)
	
	for id in to_remove:
		if selection_manager:
			selection_manager.unregister_visualizer(id)
		squad_visuals[id].queue_free()
		squad_visuals.erase(id)
	
	# Update terrain craters
	var new_craters: Array = snapshot.get("new_craters", [])
	if not new_craters.is_empty() and terrain_visualizer:
		terrain_visualizer.update_craters(new_craters)
	
	# Update destructibles
	var destructibles_list: Array = snapshot.get("destructibles", [])
	_update_destructibles(destructibles_list)

func _update_destructibles(destructibles_list: Array) -> void:
	var active_ids: Array[int] = []
	
	for dest_data in destructibles_list:
		var dest_id: int = dest_data["id"]
		active_ids.append(dest_id)
		
		# Create or update visual
		if not destructible_visuals.has(dest_id):
			_create_destructible_visual(dest_id, dest_data)
		
		_update_destructible_visual(dest_id, dest_data)
	
	# Remove visuals for destructibles that no longer exist
	var to_remove: Array[int] = []
	for id in destructible_visuals:
		if id not in active_ids:
			to_remove.append(id)
	
	for id in to_remove:
		destructible_visuals[id].queue_free()
		destructible_visuals.erase(id)

func _create_destructible_visual(dest_id: int, dest_data: Dictionary) -> void:
	var visualizer := DestructibleVisualizer.new()
	visualizer.destructible_id = dest_id
	visualizer.destructible_type = dest_data.get("dtype", "Tree")
	
	# Set position
	var x: float = dest_data.get("x", 0.0)
	var y: float = dest_data.get("y", 0.0)
	visualizer.position = Vector3(x, 0.0, y)
	
	destructibles_container.add_child(visualizer)
	destructible_visuals[dest_id] = visualizer
	
	# Connect signals
	visualizer.destroyed.connect(_on_destructible_destroyed)
	visualizer.damaged.connect(_on_destructible_damaged)

func _update_destructible_visual(dest_id: int, dest_data: Dictionary) -> void:
	var visual: DestructibleVisualizer = destructible_visuals.get(dest_id)
	if visual == null:
		return
	
	var state: String = dest_data.get("state", "Intact")
	var health: float = dest_data.get("health", 50.0)
	var health_max: float = dest_data.get("health_max", 50.0)
	
	visual.update_state(state, health, health_max)

func _on_destructible_destroyed(id: int) -> void:
	print("[Battlefield] Destructible %d destroyed" % id)

func _on_destructible_damaged(id: int) -> void:
	print("[Battlefield] Destructible %d damaged" % id)

func _create_squad_visual(squad_id: int, squad_data: Dictionary) -> void:
	var visualizer := UnitVisualizer.new()
	visualizer.squad_id = squad_id
	visualizer.faction = squad_data.get("faction", "Blue")
	visualizer.squad_size = squad_data.get("size", 12)
	
	squads_container.add_child(visualizer)
	squad_visuals[squad_id] = visualizer
	
	# Register with selection manager
	if selection_manager:
		selection_manager.register_visualizer(squad_id, visualizer)
	
	# Set initial LOD
	visualizer.set_zoom_band(current_zoom_band)

func _update_squad_visual(squad_id: int, squad_data: Dictionary) -> void:
	var visual: UnitVisualizer = squad_visuals.get(squad_id)
	if visual == null:
		return
	
	# Update position (note: simulation uses x,y but Godot uses x,z for ground plane)
	var x: float = squad_data.get("x", 0.0)
	var y: float = squad_data.get("y", 0.0)
	visual.global_position = Vector3(x, 0.0, y)
	
	# Update state
	var health: float = squad_data.get("health", 100.0)
	var health_max: float = squad_data.get("health_max", 100.0)
	var health_frac := health / health_max if health_max > 0 else 0.0
	var suppression: float = squad_data.get("suppression", 0.0)
	var morale: float = squad_data.get("morale", 1.0)
	var order: String = squad_data.get("order", "Hold")
	
	# Parse order target
	var target := Vector3.ZERO
	if "(" in order:
		var start := order.find("(")
		var end := order.find(")")
		if start != -1 and end != -1:
			var coords := order.substr(start + 1, end - start - 1)
			var parts := coords.split(",")
			if parts.size() == 2:
				target = Vector3(float(parts[0]), 0, float(parts[1]))
	
	visual.update_state(health_frac, suppression, morale, order, target)

func _on_zoom_changed(_zoom_level: float) -> void:
	_update_zoom_band()

func _update_zoom_band() -> void:
	if camera_controller:
		var new_band := camera_controller.get_zoom_band()
		if new_band != current_zoom_band:
			current_zoom_band = new_band
			_apply_zoom_band_to_all()

func _apply_zoom_band_to_all() -> void:
	for squad_id in squad_visuals:
		var visual: UnitVisualizer = squad_visuals[squad_id]
		visual.set_zoom_band(current_zoom_band)

func get_all_squad_data() -> Array:
	# Return current squad data for minimap
	var result: Array = []
	if sim_bridge:
		var snapshot := sim_bridge.get_snapshot()
		result = snapshot.get("squads", [])
	return result
