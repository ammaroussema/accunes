package com.accunes.input

import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import com.accunes.Emulator

class PhysicalGamepadHandler(private val onInputChanged: (Int) -> Unit) {

    private var physicalMask = 0

    fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        if (!isGamepadEvent(event)) return false
        val mask = mapKeyToButton(keyCode) ?: return false
        physicalMask = physicalMask or mask
        onInputChanged(physicalMask)
        return true
    }

    fun onKeyUp(keyCode: Int, event: KeyEvent): Boolean {
        if (!isGamepadEvent(event)) return false
        val mask = mapKeyToButton(keyCode) ?: return false
        physicalMask = physicalMask and mask.inv()
        onInputChanged(physicalMask)
        return true
    }

    fun onGenericMotionEvent(event: MotionEvent): Boolean {
        if (event.source and InputDevice.SOURCE_JOYSTICK != InputDevice.SOURCE_JOYSTICK) {
            return false
        }

        var dpadMask = 0

        val hatX = event.getAxisValue(MotionEvent.AXIS_HAT_X)
        val hatY = event.getAxisValue(MotionEvent.AXIS_HAT_Y)
        val stickX = event.getAxisValue(MotionEvent.AXIS_X)
        val stickY = event.getAxisValue(MotionEvent.AXIS_Y)

        val x = if (Math.abs(hatX) > 0.2f) hatX else stickX
        val y = if (Math.abs(hatY) > 0.2f) hatY else stickY

        if (x < -0.4f) dpadMask = dpadMask or Emulator.BTN_LEFT
        if (x > 0.4f) dpadMask = dpadMask or Emulator.BTN_RIGHT
        if (y < -0.4f) dpadMask = dpadMask or Emulator.BTN_UP
        if (y > 0.4f) dpadMask = dpadMask or Emulator.BTN_DOWN

        val dpadClear = (Emulator.BTN_UP or Emulator.BTN_DOWN or Emulator.BTN_LEFT or Emulator.BTN_RIGHT).inv()
        physicalMask = (physicalMask and dpadClear) or dpadMask
        onInputChanged(physicalMask)
        return true
    }

    private fun isGamepadEvent(event: KeyEvent): Boolean {
        val src = event.source
        return (src and InputDevice.SOURCE_GAMEPAD == InputDevice.SOURCE_GAMEPAD) ||
               (src and InputDevice.SOURCE_JOYSTICK == InputDevice.SOURCE_JOYSTICK)
    }

    private fun mapKeyToButton(keyCode: Int): Int? {
        return when (keyCode) {
            KeyEvent.KEYCODE_BUTTON_A -> Emulator.BTN_A
            KeyEvent.KEYCODE_BUTTON_B -> Emulator.BTN_B
            KeyEvent.KEYCODE_BUTTON_X -> Emulator.BTN_B
            KeyEvent.KEYCODE_BUTTON_Y -> Emulator.BTN_A
            KeyEvent.KEYCODE_BUTTON_SELECT -> Emulator.BTN_SELECT
            KeyEvent.KEYCODE_BUTTON_START -> Emulator.BTN_START
            KeyEvent.KEYCODE_DPAD_UP -> Emulator.BTN_UP
            KeyEvent.KEYCODE_DPAD_DOWN -> Emulator.BTN_DOWN
            KeyEvent.KEYCODE_DPAD_LEFT -> Emulator.BTN_LEFT
            KeyEvent.KEYCODE_DPAD_RIGHT -> Emulator.BTN_RIGHT
            else -> null
        }
    }
}
