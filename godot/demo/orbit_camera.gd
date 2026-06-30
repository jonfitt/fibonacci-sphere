extends Camera3D

## Orbit camera: drag to rotate, scroll to zoom (matches Bevy visualizer).
## A short left click (without drag) emits [signal viewport_clicked].

signal viewport_clicked(position: Vector2)

@export var target: Node3D
@export var distance: float = 3.0
@export var min_distance: float = 1.0
@export var max_distance: float = 10.0
@export var rotate_sensitivity: float = 0.005
@export var zoom_sensitivity: float = 0.2

const CLICK_THRESHOLD_PX := 5.0

var _yaw: float = 0.0
var _pitch: float = 0.2
var _dragging: bool = false
var _press_position: Vector2 = Vector2.ZERO


func _ready() -> void:
	_update_transform()


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		var button := event as InputEventMouseButton
		if button.button_index == MOUSE_BUTTON_LEFT:
			if button.pressed:
				_dragging = true
				_press_position = button.position
			else:
				if _dragging and _press_position.distance_to(button.position) < CLICK_THRESHOLD_PX:
					viewport_clicked.emit(button.position)
				_dragging = false
		elif button.pressed:
			if button.button_index == MOUSE_BUTTON_WHEEL_UP:
				distance = max(min_distance, distance - zoom_sensitivity)
				_update_transform()
			elif button.button_index == MOUSE_BUTTON_WHEEL_DOWN:
				distance = min(max_distance, distance + zoom_sensitivity)
				_update_transform()

	if event is InputEventMouseMotion and _dragging:
		var motion := event as InputEventMouseMotion
		_yaw -= motion.relative.x * rotate_sensitivity
		_pitch = clampf(_pitch - motion.relative.y * rotate_sensitivity, -1.4, 1.4)
		_update_transform()


func _update_transform() -> void:
	var origin := Vector3.ZERO
	if target:
		origin = target.global_position

	var offset := Vector3(
		sin(_yaw) * cos(_pitch),
		sin(_pitch),
		cos(_yaw) * cos(_pitch),
	) * distance

	global_position = origin + offset
	look_at(origin, Vector3.UP)
