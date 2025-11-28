extends Node3D

## Main.gd
## Root scene controller for This Bitter Ground.
## Manages the simulation bridge and coordinates subsystems.

@onready var camera_controller: CameraController = $CameraController
@onready var title_label: Label = $UI/TitleLabel
@onready var status_label: Label = $UI/StatusLabel
@onready var tick_label: Label = $UI/TickLabel
@onready var view_mode_label: Label = $UI/ViewModeLabel
@onready var selection_label: Label = $UI/SelectionLabel
@onready var minimap: Minimap = $UI/Minimap

# Simulation bridge
var sim_bridge: SimBridge

# Battlefield scene instance
var battlefield: Node3D

# Selection manager
var selection_manager: SelectionManager

func _ready() -> void:
	print("This Bitter Ground — Rust ECS Edition")
	print("Phase 6: Swarm AI Behavior")
	
	# Initialize simulation bridge
	sim_bridge = SimBridge.new()
	add_child(sim_bridge)
	sim_bridge.init_world()
	
	# Initialize selection manager
	selection_manager = SelectionManager.new()
	selection_manager.camera_controller = camera_controller
	selection_manager.sim_bridge = sim_bridge
	add_child(selection_manager)
	
	# Load battlefield scene
	var battlefield_scene := preload("res://scenes/Battlefield.tscn")
	battlefield = battlefield_scene.instantiate()
	add_child(battlefield)
	
	# Connect battlefield to subsystems
	if battlefield.has_method("set_sim_bridge"):
		battlefield.set_sim_bridge(sim_bridge)
	if battlefield.has_method("set_camera_controller"):
		battlefield.set_camera_controller(camera_controller)
	if battlefield.has_method("set_selection_manager"):
		battlefield.set_selection_manager(selection_manager)
	
	# Connect signals
	if camera_controller:
		camera_controller.view_mode_changed.connect(_on_view_mode_changed)
		camera_controller.zoom_changed.connect(_on_zoom_changed)
		camera_controller.camera_moved.connect(_on_camera_moved)
	
	if selection_manager:
		selection_manager.selection_changed.connect(_on_selection_changed)
		selection_manager.order_issued.connect(_on_order_issued)
	
	if minimap:
		minimap.minimap_clicked.connect(_on_minimap_clicked)
	
	_update_status()
	_update_view_mode_label()

func _process(delta: float) -> void:
	# Step simulation
	if sim_bridge:
		sim_bridge.step(delta)
		_update_tick_display()
		
		# Update battlefield visualization
		if battlefield and battlefield.has_method("update_from_snapshot"):
			var snapshot := sim_bridge.get_snapshot()
			battlefield.update_from_snapshot(snapshot)
			
			# Update minimap
			if minimap:
				minimap.update_squads(snapshot.get("squads", []))
				if camera_controller:
					minimap.update_camera_bounds(camera_controller.get_visible_bounds())

func _update_status() -> void:
	var backend := "Rust ECS" if sim_bridge and sim_bridge.is_using_rust() else "GDScript Mock"
	status_label.text = "%s Backend | Godot 4 Frontend | Phase 6" % backend

func _update_tick_display() -> void:
	if sim_bridge:
		var tick := sim_bridge.current_tick
		var time := sim_bridge.current_time
		tick_label.text = "Tick: %d | Time: %.1fs" % [tick, time]

func _update_view_mode_label() -> void:
	if camera_controller and view_mode_label:
		var mode := camera_controller._get_view_mode_name(camera_controller.current_view_mode)
		var zoom_band := camera_controller.get_zoom_band()
		view_mode_label.text = "View: %s | Zoom: %s" % [mode, zoom_band]

func _on_view_mode_changed(mode: String) -> void:
	_update_view_mode_label()

func _on_zoom_changed(_zoom_level: float) -> void:
	_update_view_mode_label()

func _on_camera_moved(_position: Vector3) -> void:
	pass  # Could update minimap camera indicator

func _on_selection_changed(selected_ids: Array[int]) -> void:
	if selection_label:
		if selected_ids.is_empty():
			selection_label.text = "No selection"
		else:
			selection_label.text = "Selected: %d squad(s)" % selected_ids.size()

func _on_order_issued(order_type: String, _squad_ids: Array[int], _target: Vector3) -> void:
	print("[Main] Order issued: %s" % order_type)

func _on_minimap_clicked(world_position: Vector3) -> void:
	if camera_controller:
		camera_controller.focus_on(world_position)

func _unhandled_input(event: InputEvent) -> void:
	# Artillery barrage controls
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_B:
				# Spawn barrage at camera focus point
				if camera_controller and sim_bridge:
					var target := camera_controller.global_position
					target.y = 0
					sim_bridge.spawn_barrage(target.x, target.z, 20.0, 8)
					print("[Main] Artillery barrage at (%.1f, %.1f)" % [target.x, target.z])
			KEY_C:
				# Spawn single crater at mouse position
				if camera_controller and sim_bridge:
					var mouse_pos := camera_controller.get_mouse_world_position()
					if mouse_pos != Vector3.ZERO:
						sim_bridge.spawn_crater(mouse_pos.x, mouse_pos.z, 4.0, 2.0)
						print("[Main] Crater at (%.1f, %.1f)" % [mouse_pos.x, mouse_pos.z])
