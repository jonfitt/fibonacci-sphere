extends Node3D

## Interactive Fibonacci sphere demo (controls match the Bevy visualizer).

const METHOD_COUNT := 6
const MIN_POINT_COUNT := 4
const MAX_POINT_COUNT := 10000
const POINT_COUNT_STEP := 10
const MIN_RADIUS := 0.2
const MAX_RADIUS := 5.0
const RADIUS_STEP := 0.1
## Radial lift for wireframe and route lines above terrain polygons.
const OVERLAY_LIFT_FRACTION := 0.004
const MOUNTAIN_THRESHOLD_STEP := 0.05
const DEEP_WATER_THRESHOLD_STEP := 0.05
const SPACING_FACTOR_STEP := 0.1
const POLAR_ICE_DISTANCE_STEP := 0.05
const POLAR_ICE_MORPHOLOGY_STEP := 0.05
const MAX_POLAR_ICE_DISTANCE := 1.5707963
const WIREFRAME_LINE_WIDTH_FRACTION := 0.0025
const PATH_LINE_WIDTH_FRACTION := 0.004
const COASTLINE_LINE_WIDTH_FRACTION := 0.0035

@export var method_index: int = 0
@export var point_count: int = 6000
@export var radius: float = 1.0
@export var show_wireframe: bool = true
@export var show_terrain_polygons: bool = true
@export var show_coastline: bool = true

@export_group("Terrain (Perlin)")
@export var terrain_seed: int = 1
@export_range(0.0, 1.0) var perlin_mountain_threshold: float = 0.55
@export_range(0.0, 1.0) var perlin_deep_water_threshold: float = 0.5
@export_range(0.01, 4.0) var perlin_spacing_factor: float = 0.2
@export_range(0.0, 1.57) var north_polar_ice_distance: float = 0.70
@export_range(0.0, 1.57) var south_polar_ice_distance: float = 0.70
@export_range(0.05, 3.0) var polar_ice_mountain_resistance: float = 0.25
@export_range(0.1, 5.0) var polar_ice_land_resistance: float = 1.0
@export_range(0.5, 12.0) var polar_ice_water_resistance: float = 2.5
@export_range(1.0, 20.0) var polar_ice_deep_water_resistance: float = 5.0
@export_range(0.0, 12.0) var polar_ice_latitude_cost: float = 2.0

@export var point_mesh: Mesh
@export var point_material: Material
@export var wireframe_material: Material
@export var coastline_material: Material
@export var path_material: Material
@export var selected_point_material: Material

@onready var _points_multimesh: MultiMeshInstance3D = (
	$Layout/HBoxContainer/SubViewportContainer/SubViewport/World/Points
)
@onready var _terrain_mesh: MeshInstance3D = (
	$Layout/HBoxContainer/SubViewportContainer/SubViewport/World/Terrain
)
@onready var _coastline_mesh: MeshInstance3D = (
	$Layout/HBoxContainer/SubViewportContainer/SubViewport/World/Coastline
)
@onready var _path_mesh: MeshInstance3D = (
	$Layout/HBoxContainer/SubViewportContainer/SubViewport/World/Path
)
@onready var _wireframe_mesh: MeshInstance3D = (
	$Layout/HBoxContainer/SubViewportContainer/SubViewport/World/Wireframe
)
@onready var _axes_mesh: MeshInstance3D = $Layout/HBoxContainer/SubViewportContainer/SubViewport/World/Axes
@onready var _camera: Camera3D = (
	$Layout/HBoxContainer/SubViewportContainer/SubViewport/World/Camera3D
)
@onready var _hud: Label = $Layout/HBoxContainer/HUDPanel/ScrollContainer/HUD
@onready var _hud_scroll: ScrollContainer = $Layout/HBoxContainer/HUDPanel/ScrollContainer
@onready var _subviewport: SubViewport = $Layout/HBoxContainer/SubViewportContainer/SubViewport
@onready var _route_panel: PanelContainer = $Layout/HBoxContainer/SubViewportContainer/RouteTerrainPanel
@onready var _cb_land: CheckBox = (
	$Layout/HBoxContainer/SubViewportContainer/RouteTerrainPanel/MarginContainer/VBoxContainer/Land
)
@onready var _cb_water: CheckBox = (
	$Layout/HBoxContainer/SubViewportContainer/RouteTerrainPanel/MarginContainer/VBoxContainer/Water
)
@onready var _cb_deep_water: CheckBox = (
	$Layout/HBoxContainer/SubViewportContainer/RouteTerrainPanel/MarginContainer/VBoxContainer/DeepWater
)
@onready var _cb_mountain: CheckBox = (
	$Layout/HBoxContainer/SubViewportContainer/RouteTerrainPanel/MarginContainer/VBoxContainer/Mountain
)
@onready var _cb_ice: CheckBox = (
	$Layout/HBoxContainer/SubViewportContainer/RouteTerrainPanel/MarginContainer/VBoxContainer/Ice
)
@onready var _cb_ice_mountain: CheckBox = (
	$Layout/HBoxContainer/SubViewportContainer/RouteTerrainPanel/MarginContainer/VBoxContainer/IceMountain
)

var _generator: FibonacciSphere
var _route_from_index: int = -1
var _route_to_index: int = -1


func _ready() -> void:
	# Godot 4.6 re-saves these on the scene file; set them at runtime to avoid .tscn churn.
	_subviewport.handle_input_locally = true
	_route_panel.set_anchors_preset(Control.PRESET_TOP_LEFT)
	_route_panel.offset_left = 12.0
	_route_panel.offset_top = 12.0
	_route_panel.offset_right = 172.0
	_route_panel.offset_bottom = 204.0
	_route_panel.mouse_filter = Control.MOUSE_FILTER_STOP

	_generator = FibonacciSphere.new()
	_camera.viewport_clicked.connect(_on_viewport_clicked)
	for checkbox in [_cb_land, _cb_water, _cb_deep_water, _cb_mountain, _cb_ice, _cb_ice_mountain]:
		checkbox.focus_mode = Control.FOCUS_NONE
		checkbox.toggled.connect(_on_route_terrain_toggled)
	_setup_points_multimesh()
	_regenerate()
	_update_hud()


func _setup_points_multimesh() -> void:
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_colors = true
	multimesh.mesh = point_mesh
	_points_multimesh.multimesh = multimesh
	if point_material:
		_points_multimesh.material_override = point_material


func _unhandled_input(event: InputEvent) -> void:
	if not event is InputEventKey or not event.pressed or event.echo:
		return

	var lattice_changed := false
	var terrain_changed := false
	var key_event := event as InputEventKey

	match key_event.keycode:
		KEY_M:
			method_index = (method_index + 1) % METHOD_COUNT
			lattice_changed = true
		KEY_EQUAL, KEY_KP_ADD:
			point_count = mini(point_count + POINT_COUNT_STEP, MAX_POINT_COUNT)
			lattice_changed = true
		KEY_MINUS, KEY_KP_SUBTRACT:
			point_count = maxi(point_count - POINT_COUNT_STEP, MIN_POINT_COUNT)
			lattice_changed = true
		KEY_BRACKETRIGHT:
			radius = minf(radius + RADIUS_STEP, MAX_RADIUS)
			lattice_changed = true
		KEY_BRACKETLEFT:
			radius = maxf(radius - RADIUS_STEP, MIN_RADIUS)
			lattice_changed = true
		KEY_COMMA:
			perlin_mountain_threshold = clampf(
				perlin_mountain_threshold - MOUNTAIN_THRESHOLD_STEP, 0.05, 0.95
			)
			terrain_changed = true
		KEY_PERIOD:
			perlin_mountain_threshold = clampf(
				perlin_mountain_threshold + MOUNTAIN_THRESHOLD_STEP, 0.05, 0.95
			)
			terrain_changed = true
		KEY_9:
			perlin_deep_water_threshold = clampf(
				perlin_deep_water_threshold - DEEP_WATER_THRESHOLD_STEP, 0.05, 0.95
			)
			terrain_changed = true
		KEY_0:
			perlin_deep_water_threshold = clampf(
				perlin_deep_water_threshold + DEEP_WATER_THRESHOLD_STEP, 0.05, 0.95
			)
			terrain_changed = true
		KEY_SEMICOLON:
			perlin_spacing_factor = clampf(
				perlin_spacing_factor - SPACING_FACTOR_STEP, 0.1, 5.0
			)
			terrain_changed = true
		KEY_APOSTROPHE:
			perlin_spacing_factor = clampf(
				perlin_spacing_factor + SPACING_FACTOR_STEP, 0.1, 5.0
			)
			terrain_changed = true
		KEY_1:
			north_polar_ice_distance = clampf(
				north_polar_ice_distance - POLAR_ICE_DISTANCE_STEP, 0.0, MAX_POLAR_ICE_DISTANCE
			)
			terrain_changed = true
		KEY_2:
			north_polar_ice_distance = clampf(
				north_polar_ice_distance + POLAR_ICE_DISTANCE_STEP, 0.0, MAX_POLAR_ICE_DISTANCE
			)
			terrain_changed = true
		KEY_3:
			south_polar_ice_distance = clampf(
				south_polar_ice_distance - POLAR_ICE_DISTANCE_STEP, 0.0, MAX_POLAR_ICE_DISTANCE
			)
			terrain_changed = true
		KEY_4:
			south_polar_ice_distance = clampf(
				south_polar_ice_distance + POLAR_ICE_DISTANCE_STEP, 0.0, MAX_POLAR_ICE_DISTANCE
			)
			terrain_changed = true
		KEY_5:
			polar_ice_mountain_resistance = clampf(
				polar_ice_mountain_resistance - 0.1, 0.05, 3.0
			)
			terrain_changed = true
		KEY_6:
			polar_ice_mountain_resistance = clampf(
				polar_ice_mountain_resistance + 0.1, 0.05, 3.0
			)
			terrain_changed = true
		KEY_7:
			polar_ice_water_resistance = clampf(
				polar_ice_water_resistance - 0.1, 0.5, 12.0
			)
			terrain_changed = true
		KEY_8:
			polar_ice_water_resistance = clampf(
				polar_ice_water_resistance + 0.1, 0.5, 12.0
			)
			terrain_changed = true
		KEY_Z:
			polar_ice_latitude_cost = clampf(
				polar_ice_latitude_cost - 0.25, 0.0, 12.0
			)
			terrain_changed = true
		KEY_X:
			polar_ice_latitude_cost = clampf(
				polar_ice_latitude_cost + 0.25, 0.0, 12.0
			)
			terrain_changed = true
		KEY_R:
			terrain_seed += 1
			terrain_changed = true
		KEY_H:
			show_wireframe = not show_wireframe
			_update_wireframe()
			_update_hud()
		KEY_ESCAPE:
			_clear_route()
			_update_hud()

	if lattice_changed:
		_regenerate()
		_update_hud()
	elif terrain_changed:
		_regenerate_terrain()


func _on_route_terrain_toggled(_pressed: bool) -> void:
	if _route_from_index >= 0 and _route_to_index >= 0:
		_update_path()


func _on_viewport_clicked(viewport_position: Vector2) -> void:
	var vertex_index := _pick_vertex_index(viewport_position)
	if vertex_index < 0:
		return

	if _route_from_index < 0 or (_route_from_index >= 0 and _route_to_index >= 0):
		_route_from_index = vertex_index
		_route_to_index = -1
	elif _route_from_index >= 0:
		_route_to_index = vertex_index

	_update_point_highlights()
	if _route_from_index >= 0 and _route_to_index >= 0:
		_update_path()
	_update_hud()


func _pick_vertex_index(viewport_position: Vector2) -> int:
	var ray_origin := _camera.project_ray_origin(viewport_position)
	var ray_dir := _camera.project_ray_normal(viewport_position)
	var hit := _intersect_sphere(ray_origin, ray_dir, Vector3.ZERO, radius)
	if not hit.is_finite():
		return -1
	return _generator.find_nearest_vertex_index(hit)


func _intersect_sphere(
	ray_origin: Vector3,
	ray_dir: Vector3,
	sphere_center: Vector3,
	sphere_radius: float,
) -> Vector3:
	var oc := ray_origin - sphere_center
	var a := ray_dir.dot(ray_dir)
	var b := 2.0 * oc.dot(ray_dir)
	var c := oc.dot(oc) - sphere_radius * sphere_radius
	var discriminant := b * b - 4.0 * a * c
	if discriminant < 0.0:
		return Vector3.INF

	var sqrt_disc := sqrt(discriminant)
	var t := (-b - sqrt_disc) / (2.0 * a)
	if t < 0.0:
		t = (-b + sqrt_disc) / (2.0 * a)
	if t < 0.0:
		return Vector3.INF

	return ray_origin + ray_dir * t


func _regenerate() -> void:
	_clear_route()

	var positions: PackedVector3Array = _generator.generate_with_terrain(
		method_index,
		point_count,
		radius,
		terrain_seed,
		perlin_mountain_threshold,
		perlin_deep_water_threshold,
		perlin_spacing_factor,
		north_polar_ice_distance,
		south_polar_ice_distance,
		polar_ice_mountain_resistance,
		polar_ice_land_resistance,
		polar_ice_water_resistance,
		polar_ice_deep_water_resistance,
		polar_ice_latitude_cost,
	)
	if positions.is_empty():
		_clear_terrain_visuals()
		_points_multimesh.multimesh.instance_count = 0
		return

	_update_points()
	_update_terrain()
	_update_coastline()
	_update_wireframe()
	_update_axes()
	_update_path()


func _regenerate_terrain() -> void:
	if _generator.get_point_count() == 0:
		return

	if not _generator.generate_terrain(
		terrain_seed,
		perlin_mountain_threshold,
		perlin_deep_water_threshold,
		perlin_spacing_factor,
		north_polar_ice_distance,
		south_polar_ice_distance,
		polar_ice_mountain_resistance,
		polar_ice_land_resistance,
		polar_ice_water_resistance,
		polar_ice_deep_water_resistance,
		polar_ice_latitude_cost,
	):
		push_error("Terrain generation failed")
		_clear_terrain_visuals()
		return

	_clear_route()
	_update_terrain()
	_update_coastline()
	_update_hud()


func _clear_terrain_visuals() -> void:
	_terrain_mesh.mesh = null
	_coastline_mesh.mesh = null


func _overlay_lift_distance() -> float:
	return radius * OVERLAY_LIFT_FRACTION


func _default_point_color() -> Color:
	if point_material is StandardMaterial3D:
		return (point_material as StandardMaterial3D).albedo_color
	return Color(1.0, 0.85, 0.2)


func _selected_point_color() -> Color:
	if selected_point_material is StandardMaterial3D:
		return (selected_point_material as StandardMaterial3D).albedo_color
	return Color(1.0, 0.95, 0.35)


func _update_points() -> void:
	var multimesh := _points_multimesh.multimesh
	if multimesh == null:
		_setup_points_multimesh()
		multimesh = _points_multimesh.multimesh

	_generator.populate_point_multimesh(
		multimesh,
		_overlay_lift_distance(),
		_default_point_color(),
	)
	_update_point_highlights()


func _apply_prominent_overlay_material(
	mesh_instance: MeshInstance3D,
	mesh: ArrayMesh,
	material: Material,
	render_priority: int,
) -> void:
	mesh_instance.mesh = mesh
	if material == null:
		return

	var overlay_material := material.duplicate()
	if overlay_material is StandardMaterial3D:
		var standard := overlay_material as StandardMaterial3D
		standard.render_priority = render_priority
		standard.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		standard.emission_enabled = true
		standard.emission = standard.albedo_color
		standard.emission_energy_multiplier = 1.4
	mesh_instance.material_override = overlay_material


func _mesh_from_arrays(vertices: PackedVector3Array, indices: PackedInt32Array) -> ArrayMesh:
	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


func _update_terrain() -> void:
	_terrain_mesh.visible = show_terrain_polygons
	if not show_terrain_polygons:
		_terrain_mesh.mesh = null
		return

	var mesh_data: Array = _generator.get_terrain_mesh_data()
	if mesh_data.size() < 4:
		_terrain_mesh.mesh = null
		return

	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = mesh_data[0]
	arrays[Mesh.ARRAY_COLOR] = mesh_data[1]
	arrays[Mesh.ARRAY_NORMAL] = mesh_data[2]
	arrays[Mesh.ARRAY_INDEX] = mesh_data[3]

	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)

	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.vertex_color_use_as_albedo = true
	material.cull_mode = BaseMaterial3D.CULL_BACK

	_terrain_mesh.mesh = mesh
	_terrain_mesh.material_override = material


func _update_coastline() -> void:
	_coastline_mesh.visible = show_coastline
	if not show_coastline:
		_coastline_mesh.mesh = null
		return

	var segments := _generator.get_coastline_segments()
	if segments.is_empty():
		_coastline_mesh.mesh = null
		return

	var mesh_data: Array = FibonacciSphere.build_ribbon_line_mesh(
		segments,
		radius * COASTLINE_LINE_WIDTH_FRACTION,
		_overlay_lift_distance(),
	)
	if mesh_data.is_empty():
		_coastline_mesh.mesh = null
		return

	var mesh := _mesh_from_arrays(mesh_data[0], mesh_data[1])
	_apply_prominent_overlay_material(_coastline_mesh, mesh, coastline_material, 2)


func _allowed_terrain_types() -> PackedInt32Array:
	var allowed := PackedInt32Array()
	if _cb_land.button_pressed:
		allowed.append(FibonacciTerrainType.LAND)
	if _cb_water.button_pressed:
		allowed.append(FibonacciTerrainType.WATER)
	if _cb_deep_water.button_pressed:
		allowed.append(FibonacciTerrainType.DEEP_WATER)
	if _cb_mountain.button_pressed:
		allowed.append(FibonacciTerrainType.MOUNTAIN)
	if _cb_ice.button_pressed:
		allowed.append(FibonacciTerrainType.ICE)
	if _cb_ice_mountain.button_pressed:
		allowed.append(FibonacciTerrainType.ICE_MOUNTAIN)
	return allowed


func _update_path() -> void:
	if _route_from_index < 0 or _route_to_index < 0:
		_path_mesh.mesh = null
		return

	var allowed := _allowed_terrain_types()
	if allowed.is_empty():
		_path_mesh.mesh = null
		return

	var path_positions: PackedVector3Array = (
		_generator.shortest_surface_path_positions_with_allowed_terrain(
			_route_from_index, _route_to_index, allowed
		)
	)

	if path_positions.size() < 2:
		_path_mesh.mesh = null
		return

	var segments := PackedVector3Array()
	for index in path_positions.size() - 1:
		segments.append(path_positions[index])
		segments.append(path_positions[index + 1])

	var mesh_data: Array = FibonacciSphere.build_ribbon_line_mesh(
		segments,
		radius * PATH_LINE_WIDTH_FRACTION,
		_overlay_lift_distance(),
	)
	if mesh_data.is_empty():
		_path_mesh.mesh = null
		return

	var mesh := _mesh_from_arrays(mesh_data[0], mesh_data[1])
	_apply_prominent_overlay_material(_path_mesh, mesh, path_material, 3)


func _clear_route() -> void:
	_route_from_index = -1
	_route_to_index = -1
	_path_mesh.mesh = null
	_update_point_highlights()


func _update_point_highlights() -> void:
	var multimesh := _points_multimesh.multimesh
	if multimesh == null:
		return

	_generator.update_point_multimesh_highlights(
		multimesh,
		_route_from_index,
		_route_to_index,
		_default_point_color(),
		_selected_point_color(),
	)


func _update_axes() -> void:
	var length := radius * 1.5 + 0.15
	var origin := Vector3.ZERO
	var verts := PackedVector3Array([
		origin,
		Vector3(length, 0.0, 0.0),
		origin,
		Vector3(0.0, length, 0.0),
		origin,
		Vector3(0.0, 0.0, length),
	])
	var colors := PackedColorArray([
		Color(0.95, 0.25, 0.25),
		Color(0.95, 0.25, 0.25),
		Color(0.25, 0.9, 0.3),
		Color(0.25, 0.9, 0.3),
		Color(0.35, 0.55, 1.0),
		Color(0.35, 0.55, 1.0),
	])

	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_COLOR] = colors

	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_LINES, arrays)
	_axes_mesh.mesh = mesh


func _update_wireframe() -> void:
	_wireframe_mesh.visible = show_wireframe
	if not show_wireframe:
		_wireframe_mesh.mesh = null
		return

	var segments := _generator.get_wireframe_segments()
	if segments.is_empty():
		_wireframe_mesh.mesh = null
		return

	var mesh_data: Array = FibonacciSphere.build_ribbon_line_mesh(
		segments,
		radius * WIREFRAME_LINE_WIDTH_FRACTION,
		_overlay_lift_distance(),
	)
	if mesh_data.is_empty():
		_wireframe_mesh.mesh = null
		return

	var mesh := _mesh_from_arrays(mesh_data[0], mesh_data[1])
	_apply_prominent_overlay_material(_wireframe_mesh, mesh, wireframe_material, 2)


func _route_status_text() -> String:
	if _route_from_index < 0:
		return "Route: click a node to set start"
	if _route_to_index < 0:
		return "Route: click a second node (Esc to clear)"
	var allowed := _allowed_terrain_types()
	if allowed.is_empty():
		return "Route: select at least one terrain type"
	var length := _generator.shortest_surface_path_length_with_allowed_terrain(
		_route_from_index, _route_to_index, allowed
	)
	if length < 0.0:
		return "Route: no path with selected terrain types"
	return "Route: %d → %d  length %.3f" % [_route_from_index, _route_to_index, length]


func _update_hud() -> void:
	var method_text: String = FibonacciSphere.get_method_description(method_index)
	var land_percent := perlin_mountain_threshold * 100.0
	var mountain_percent := (1.0 - perlin_mountain_threshold) * 100.0
	var shallow_percent := (1.0 - perlin_deep_water_threshold) * 100.0
	var deep_percent := perlin_deep_water_threshold * 100.0
	_hud.text = (
		"%s\n\n---\nPoints: %d  Radius: %.1f  Wireframe: %s\n"
		% [
			method_text,
			_generator.get_point_count(),
			_generator.get_radius(),
			"on" if show_wireframe else "off",
		]
		+ "Terrain: %s  Coastline: %s  Seed: %d\n" % [
			"on" if show_terrain_polygons else "off",
			"on" if show_coastline else "off",
			terrain_seed,
		]
		+ "Perlin mountain split: %.0f%% land / %.0f%% mountain (above sea)\n"
		% [land_percent, mountain_percent]
		+ "Perlin deep-water split: %.0f%% shallow / %.0f%% deep (below sea)\n"
		% [shallow_percent, deep_percent]
		+ "Perlin spacing factor: %.2f\n" % perlin_spacing_factor
		+ "Polar ice distance (N/S): %.2f / %.2f rad\n"
		% [north_polar_ice_distance, south_polar_ice_distance]
		+ "Polar ice flood (mountain / water / latitude cost): %.2f / %.2f / %.2f\n"
		% [
			polar_ice_mountain_resistance,
			polar_ice_water_resistance,
			polar_ice_latitude_cost,
		]
		+ "%s\n" % _route_status_text()
		+ "\nAxes: Y-up (RGB = XYZ)\n\n"
		+ "M: method  +/-: count  [/]: radius  H: wireframe  R: new seed  Esc: clear route\n"
		+ ",/.: mountain split  9/0: deep-water split  ;/': spacing factor\n"
		+ "1/2: north polar ice  3/4: south polar ice  5/6: mountain resist  7/8: water resist  Z/X: latitude cost\n"
		+ "Click nodes in the 3D view to route (drag to orbit, scroll to zoom)"
	)
	call_deferred("_reset_hud_scroll")


func _reset_hud_scroll() -> void:
	_hud_scroll.scroll_vertical = 0
