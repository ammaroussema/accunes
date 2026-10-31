package com.accunes.input

import android.content.Context
import android.os.Build
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.accunes.Emulator

class TouchControllerState(context: Context) {

    private val vibrator: Vibrator? = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        val manager = context.getSystemService(Context.VIBRATOR_MANAGER_SERVICE) as? VibratorManager
        manager?.defaultVibrator
    } else {
        @Suppress("DEPRECATION")
        context.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator
    }

    var buttonsMask by mutableIntStateOf(0)
    var turboA by mutableStateOf(false)
    var turboB by mutableStateOf(false)
    var hapticsEnabled by mutableStateOf(true)
    var zapperMode by mutableStateOf(false)
    var opacity by mutableFloatStateOf(0.75f)
    var isPhysicalConnected by mutableStateOf(false)

    private var turboCounter = 0

    fun pressButton(mask: Int) {
        if ((buttonsMask and mask) == 0) {
            triggerHaptic()
        }
        buttonsMask = buttonsMask or mask
    }

    fun releaseButton(mask: Int) {
        buttonsMask = buttonsMask and mask.inv()
    }

    fun setDpadDirection(mask: Int) {
        val dpadClear = (Emulator.BTN_UP or Emulator.BTN_DOWN or Emulator.BTN_LEFT or Emulator.BTN_RIGHT).inv()
        val oldDpad = buttonsMask and (Emulator.BTN_UP or Emulator.BTN_DOWN or Emulator.BTN_LEFT or Emulator.BTN_RIGHT)
        if (oldDpad != mask && mask != 0) {
            triggerHaptic()
        }
        buttonsMask = (buttonsMask and dpadClear) or mask
    }

    fun releaseDpad() {
        val dpadClear = (Emulator.BTN_UP or Emulator.BTN_DOWN or Emulator.BTN_LEFT or Emulator.BTN_RIGHT).inv()
        buttonsMask = buttonsMask and dpadClear
    }

    fun tickTurbo(): Int {
        turboCounter = (turboCounter + 1) % 2
        var active = buttonsMask
        if (turboA) {
            active = if (turboCounter == 0) active or Emulator.BTN_A else active and Emulator.BTN_A.inv()
        }
        if (turboB) {
            active = if (turboCounter == 0) active or Emulator.BTN_B else active and Emulator.BTN_B.inv()
        }
        return active
    }

    fun triggerHaptic() {
        if (!hapticsEnabled || vibrator == null) return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            vibrator.vibrate(VibrationEffect.createPredefined(VibrationEffect.EFFECT_CLICK))
        } else {
            @Suppress("DEPRECATION")
            vibrator.vibrate(10)
        }
    }
}
