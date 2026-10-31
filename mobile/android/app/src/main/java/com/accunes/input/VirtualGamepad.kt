package com.accunes.input

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.accunes.Emulator
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.sin
import kotlin.math.sqrt

@Composable
fun VirtualGamepad(
    state: TouchControllerState,
    modifier: Modifier = Modifier
) {
    val targetAlpha = if (state.isPhysicalConnected) 0.0f else state.opacity
    val animatedAlpha by animateFloatAsState(targetValue = targetAlpha, label = "gamepad_alpha")

    if (animatedAlpha <= 0.01f) return

    Box(
        modifier = modifier
            .fillMaxSize()
            .alpha(animatedAlpha)
            .padding(16.dp)
    ) {
        if (!state.zapperMode) {
            Dpad(
                modifier = Modifier
                    .align(Alignment.BottomStart)
                    .size(170.dp),
                onDirectionChange = { state.setDpadDirection(it) },
                onRelease = { state.releaseDpad() }
            )
        } else {
            RoundButton(
                text = "AWAY",
                color = Color(0x99555555),
                modifier = Modifier.align(Alignment.BottomStart).size(70.dp)
            ) { pressed ->
                if (pressed) {
                    state.pressButton(Emulator.BTN_B)
                } else {
                    state.releaseButton(Emulator.BTN_B)
                }
            }
        }

        Row(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .padding(bottom = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            PillButton(text = "SELECT") { pressed ->
                if (pressed) state.pressButton(Emulator.BTN_SELECT) else state.releaseButton(Emulator.BTN_SELECT)
            }
            PillButton(text = "START") { pressed ->
                if (pressed) state.pressButton(Emulator.BTN_START) else state.releaseButton(Emulator.BTN_START)
            }
        }

        ActionButtons(
            state = state,
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .size(180.dp)
        )
    }
}

@Composable
fun Dpad(
    modifier: Modifier = Modifier,
    onDirectionChange: (Int) -> Unit,
    onRelease: () -> Unit
) {
    var knobOffset by remember { mutableStateOf(Offset.Zero) }

    Box(
        modifier = modifier
            .clip(CircleShape)
            .background(Color(0x33FFFFFF))
            .pointerInput(Unit) {
                detectDragGestures(
                    onDragStart = { offset ->
                        val center = Offset(size.width / 2f, size.height / 2f)
                        val delta = offset - center
                        knobOffset = updateDpad(delta, size.width / 2f, onDirectionChange)
                    },
                    onDrag = { change, _ ->
                        change.consume()
                        val center = Offset(size.width / 2f, size.height / 2f)
                        val delta = change.position - center
                        knobOffset = updateDpad(delta, size.width / 2f, onDirectionChange)
                    },
                    onDragEnd = {
                        knobOffset = Offset.Zero
                        onRelease()
                    },
                    onDragCancel = {
                        knobOffset = Offset.Zero
                        onRelease()
                    }
                )
            },
        contentAlignment = Alignment.Center
    ) {
        Canvas(modifier = Modifier.fillMaxSize()) {
            val c = center
            val r = size.minDimension / 2f

            drawCircle(Color(0x22FFFFFF), radius = r)
            drawCircle(Color(0x44FFFFFF), radius = r * 0.45f)
            drawCircle(Color(0xFFE52521), radius = r * 0.28f, center = c + knobOffset)
        }
    }
}

private fun updateDpad(delta: Offset, maxRadius: Float, onDirectionChange: (Int) -> Unit): Offset {
    val dist = sqrt(delta.x * delta.x + delta.y * delta.y)
    val deadzone = maxRadius * 0.15f
    if (dist < deadzone) {
        onDirectionChange(0)
        return Offset.Zero
    }

    val angle = Math.toDegrees(atan2(delta.y.toDouble(), delta.x.toDouble())).let {
        if (it < 0) it + 360.0 else it
    }

    val mask = when (angle) {
        in 22.5..67.5 -> Emulator.BTN_DOWN or Emulator.BTN_RIGHT
        in 67.5..112.5 -> Emulator.BTN_DOWN
        in 112.5..157.5 -> Emulator.BTN_DOWN or Emulator.BTN_LEFT
        in 157.5..202.5 -> Emulator.BTN_LEFT
        in 202.5..247.5 -> Emulator.BTN_UP or Emulator.BTN_LEFT
        in 247.5..292.5 -> Emulator.BTN_UP
        in 292.5..337.5 -> Emulator.BTN_UP or Emulator.BTN_RIGHT
        else -> Emulator.BTN_RIGHT
    }

    onDirectionChange(mask)

    val clampedDist = dist.coerceAtMost(maxRadius * 0.6f)
    val rad = Math.toRadians(angle)
    return Offset((clampedDist * cos(rad)).toFloat(), (clampedDist * sin(rad)).toFloat())
}

@Composable
fun ActionButtons(
    state: TouchControllerState,
    modifier: Modifier = Modifier
) {
    Box(modifier = modifier) {
        RoundButton(
            text = "TB",
            color = Color(0x66FF5722),
            modifier = Modifier.align(Alignment.TopStart).size(48.dp)
        ) { state.turboB = it }

        RoundButton(
            text = "TA",
            color = Color(0x66FF5722),
            modifier = Modifier.align(Alignment.TopEnd).size(48.dp)
        ) { state.turboA = it }

        RoundButton(
            text = "B",
            color = Color(0xFFE52521),
            modifier = Modifier.align(Alignment.BottomStart).size(62.dp)
        ) { pressed ->
            if (pressed) state.pressButton(Emulator.BTN_B) else state.releaseButton(Emulator.BTN_B)
        }

        RoundButton(
            text = "A",
            color = Color(0xFFE52521),
            modifier = Modifier.align(Alignment.BottomEnd).size(62.dp)
        ) { pressed ->
            if (pressed) state.pressButton(Emulator.BTN_A) else state.releaseButton(Emulator.BTN_A)
        }

        RoundButton(
            text = "B+A",
            color = Color(0x88333333),
            modifier = Modifier.align(Alignment.Center).size(42.dp)
        ) { pressed ->
            if (pressed) {
                state.pressButton(Emulator.BTN_A or Emulator.BTN_B)
            } else {
                state.releaseButton(Emulator.BTN_A or Emulator.BTN_B)
            }
        }
    }
}

@Composable
fun RoundButton(
    text: String,
    color: Color,
    modifier: Modifier = Modifier,
    onStateChange: (Boolean) -> Unit
) {
    var isPressed by remember { mutableStateOf(false) }

    Box(
        modifier = modifier
            .clip(CircleShape)
            .background(if (isPressed) color.copy(alpha = 1.0f) else color)
            .pointerInput(Unit) {
                detectTapGestures(
                    onPress = {
                        isPressed = true
                        onStateChange(true)
                        tryAwaitRelease()
                        isPressed = false
                        onStateChange(false)
                    }
                )
            },
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = text,
            color = Color.White,
            fontWeight = FontWeight.Bold,
            fontSize = 14.sp
        )
    }
}

@Composable
fun PillButton(
    text: String,
    onStateChange: (Boolean) -> Unit
) {
    var isPressed by remember { mutableStateOf(false) }

    Box(
        modifier = Modifier
            .width(64.dp)
            .height(26.dp)
            .clip(RoundedCornerShape(13.dp))
            .background(if (isPressed) Color(0xFF666666) else Color(0x55333333))
            .pointerInput(Unit) {
                detectTapGestures(
                    onPress = {
                        isPressed = true
                        onStateChange(true)
                        tryAwaitRelease()
                        isPressed = false
                        onStateChange(false)
                    }
                )
            },
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = text,
            color = Color.White,
            fontWeight = FontWeight.Bold,
            fontSize = 10.sp
        )
    }
}
