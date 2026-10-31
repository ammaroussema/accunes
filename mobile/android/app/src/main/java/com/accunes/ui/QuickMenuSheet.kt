package com.accunes.ui

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.accunes.EmulationLoop
import com.accunes.Emulator
import com.accunes.input.TouchControllerState
import com.accunes.storage.SaveStorage
import com.accunes.video.NesSurfaceView

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun QuickMenuSheet(
    emulator: Emulator,
    emulationLoop: EmulationLoop,
    surfaceView: NesSurfaceView,
    controllerState: TouchControllerState,
    saveStorage: SaveStorage,
    romHash: String,
    onDismiss: () -> Unit,
    onExitGame: () -> Unit
) {
    var selectedSlot by remember { mutableIntStateOf(0) }
    var statusMsg by remember { mutableStateOf<String?>(null) }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Text(
                text = "Quick Menu",
                style = MaterialTheme.typography.titleLarge
            )

            statusMsg?.let {
                Text(
                    text = it,
                    color = MaterialTheme.colorScheme.primary,
                    fontSize = 14.sp
                )
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                for (slot in 0..4) {
                    val hasSave = saveStorage.hasStateSlot(romHash, slot)
                    FilterChip(
                        selected = selectedSlot == slot,
                        onClick = { selectedSlot = slot },
                        label = { Text("Slot $slot" + if (hasSave) " *" else "") }
                    )
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                Button(
                    onClick = {
                        val ok = saveStorage.saveStateSlot(emulator, romHash, selectedSlot)
                        statusMsg = if (ok) "Saved to Slot $selectedSlot" else "Save failed"
                    },
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Save State")
                }

                Button(
                    onClick = {
                        val ok = saveStorage.loadStateSlot(emulator, romHash, selectedSlot)
                        statusMsg = if (ok) "Loaded Slot $selectedSlot" else "Slot empty"
                    },
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Load State")
                }
            }

            HorizontalDivider()

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                OutlinedButton(
                    onClick = {
                        emulator.reset()
                        statusMsg = "Reset"
                    },
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Soft Reset")
                }

                OutlinedButton(
                    onClick = {
                        emulator.powerCycle()
                        statusMsg = "Power Cycled"
                    },
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Hard Reset")
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Fast Forward (2x)")
                Switch(
                    checked = emulationLoop.isFastForward,
                    onCheckedChange = { emulationLoop.isFastForward = it }
                )
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Touch Zapper Gun Mode")
                Switch(
                    checked = controllerState.zapperMode,
                    onCheckedChange = { controllerState.zapperMode = it }
                )
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Haptic Feedback")
                Switch(
                    checked = controllerState.hapticsEnabled,
                    onCheckedChange = { controllerState.hapticsEnabled = it }
                )
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Smooth Filtering")
                Switch(
                    checked = surfaceView.renderer.isLinearFilter,
                    onCheckedChange = { surfaceView.renderer.isLinearFilter = it }
                )
            }

            Column {
                Text("Touch Controls Opacity: ${(controllerState.opacity * 100).toInt()}%")
                Slider(
                    value = controllerState.opacity,
                    onValueChange = { controllerState.opacity = it },
                    valueRange = 0.1f..1.0f
                )
            }

            Button(
                onClick = onExitGame,
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
                modifier = Modifier.fillMaxWidth()
            ) {
                Text("Exit to Menu")
            }
        }
    }
}
