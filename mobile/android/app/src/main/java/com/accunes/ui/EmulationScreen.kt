package com.accunes.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.accunes.EmulationLoop
import com.accunes.Emulator
import com.accunes.input.TouchControllerState
import com.accunes.input.VirtualGamepad
import com.accunes.storage.SaveStorage
import com.accunes.video.NesSurfaceView

@Composable
fun EmulationScreen(
    emulator: Emulator,
    emulationLoop: EmulationLoop,
    surfaceView: NesSurfaceView,
    controllerState: TouchControllerState,
    saveStorage: SaveStorage,
    romHash: String,
    onExitGame: () -> Unit
) {
    var showQuickMenu by remember { mutableStateOf(false) }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black)
    ) {
        AndroidView(
            factory = { surfaceView },
            modifier = Modifier
                .fillMaxSize()
                .pointerInput(controllerState.zapperMode) {
                    if (controllerState.zapperMode) {
                        detectTapGestures(
                            onPress = { offset ->
                                val targetRatio = 4.0f / 3.0f
                                val currentRatio = size.width.toFloat() / size.height.toFloat()
                                var vpX = 0f
                                var vpY = 0f
                                var vpWidth = size.width.toFloat()
                                var vpHeight = size.height.toFloat()

                                if (currentRatio > targetRatio) {
                                    vpWidth = size.height.toFloat() * targetRatio
                                    vpX = (size.width.toFloat() - vpWidth) / 2f
                                } else {
                                    vpHeight = size.width.toFloat() / targetRatio
                                    vpY = (size.height.toFloat() - vpHeight) / 2f
                                }

                                val relX = ((offset.x - vpX) / vpWidth * 256f).toInt().coerceIn(0, 255)
                                val relY = ((offset.y - vpY) / vpHeight * 240f).toInt().coerceIn(0, 239)

                                controllerState.triggerHaptic()
                                emulator.setZapper(relX, relY, true)
                                tryAwaitRelease()
                                emulator.setZapper(relX, relY, false)
                            }
                        )
                    }
                }
        )

        VirtualGamepad(
            state = controllerState,
            modifier = Modifier.fillMaxSize()
        )

        IconButton(
            onClick = {
                emulationLoop.pause()
                showQuickMenu = true
            },
            modifier = Modifier
                .align(Alignment.TopEnd)
                .statusBarsPadding()
                .padding(8.dp)
        ) {
            Icon(
                Icons.Default.Menu,
                contentDescription = "Menu",
                tint = Color.White.copy(alpha = 0.7f)
            )
        }

        if (showQuickMenu) {
            QuickMenuSheet(
                emulator = emulator,
                emulationLoop = emulationLoop,
                surfaceView = surfaceView,
                controllerState = controllerState,
                saveStorage = saveStorage,
                romHash = romHash,
                onDismiss = {
                    showQuickMenu = false
                    emulationLoop.resume()
                },
                onExitGame = {
                    showQuickMenu = false
                    onExitGame()
                }
            )
        }
    }
}
