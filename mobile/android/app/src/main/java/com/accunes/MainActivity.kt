package com.accunes

import android.net.Uri
import android.os.Bundle
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.*
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.accunes.audio.AudioEngine
import com.accunes.input.PhysicalGamepadHandler
import com.accunes.input.TouchControllerState
import com.accunes.storage.SaveStorage
import com.accunes.ui.EmulationScreen
import com.accunes.ui.RecentRom
import com.accunes.ui.RomPickerScreen
import com.accunes.ui.theme.AccuNESTheme
import com.accunes.video.NesSurfaceView

class MainActivity : ComponentActivity() {

    private var emulator: Emulator? = null
    private var surfaceView: NesSurfaceView? = null
    private var audioEngine: AudioEngine? = null
    private var emulationLoop: EmulationLoop? = null
    private lateinit var controllerState: TouchControllerState
    private lateinit var saveStorage: SaveStorage
    private lateinit var gamepadHandler: PhysicalGamepadHandler

    private var currentRomHash: String = ""

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        hideSystemUI()

        controllerState = TouchControllerState(this)
        saveStorage = SaveStorage(this)
        gamepadHandler = PhysicalGamepadHandler { buttons ->
            emulationLoop?.physicalButtons = buttons
            controllerState.isPhysicalConnected = (buttons != 0)
        }

        setContent {
            AccuNESTheme {
                var activeRomTitle by remember { mutableStateOf<String?>(null) }
                val recentRoms = remember { mutableStateListOf<RecentRom>() }

                if (activeRomTitle != null && emulator != null && emulationLoop != null && surfaceView != null) {
                    EmulationScreen(
                        emulator = emulator!!,
                        emulationLoop = emulationLoop!!,
                        surfaceView = surfaceView!!,
                        controllerState = controllerState,
                        saveStorage = saveStorage,
                        romHash = currentRomHash,
                        onExitGame = {
                            stopEmulation()
                            activeRomTitle = null
                        }
                    )
                } else {
                    RomPickerScreen(
                        recentRoms = recentRoms,
                        onRomSelected = { uri, title ->
                            loadRomFromUri(uri, title)
                            if (!recentRoms.any { it.uri == uri }) {
                                recentRoms.add(0, RecentRom(title, uri))
                            }
                            activeRomTitle = title
                        }
                    )
                }
            }
        }
    }

    private fun loadRomFromUri(uri: Uri, @Suppress("UNUSED_PARAMETER") title: String) {
        stopEmulation()

        val romBytes = contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: return

        val emu = Emulator(44100.0)
        if (!emu.loadRom(romBytes)) {
            emu.close()
            return
        }

        val crc = emu.getRomCrc()
        currentRomHash = String.format("%08X", crc)

        saveStorage.loadSram(emu, currentRomHash)

        val sv = NesSurfaceView(this)
        val audio = AudioEngine(44100)
        val loop = EmulationLoop(emu, sv, audio, controllerState)

        emulator = emu
        surfaceView = sv
        audioEngine = audio
        emulationLoop = loop

        loop.start()
    }

    private fun stopEmulation() {
        val emu = emulator
        if (emu != null && currentRomHash.isNotEmpty()) {
            saveStorage.saveSram(emu, currentRomHash)
        }
        emulationLoop?.stop()
        surfaceView?.onPause()
        emulator?.close()

        emulator = null
        emulationLoop = null
        surfaceView = null
        audioEngine = null
    }

    override fun onPause() {
        super.onPause()
        emulationLoop?.pause()
        surfaceView?.onPause()
        val emu = emulator
        if (emu != null && currentRomHash.isNotEmpty()) {
            saveStorage.saveSram(emu, currentRomHash)
        }
    }

    override fun onResume() {
        super.onResume()
        hideSystemUI()
        surfaceView?.onResume()
        emulationLoop?.resume()
    }

    override fun onDestroy() {
        stopEmulation()
        super.onDestroy()
    }

    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        if (event.action == KeyEvent.ACTION_DOWN) {
            if (gamepadHandler.onKeyDown(event.keyCode, event)) return true
        } else if (event.action == KeyEvent.ACTION_UP) {
            if (gamepadHandler.onKeyUp(event.keyCode, event)) return true
        }
        return super.dispatchKeyEvent(event)
    }

    override fun dispatchGenericMotionEvent(ev: MotionEvent): Boolean {
        if (gamepadHandler.onGenericMotionEvent(ev)) return true
        return super.dispatchGenericMotionEvent(ev)
    }

    private fun hideSystemUI() {
        WindowCompat.setDecorFitsSystemWindows(window, false)
        val controller = WindowCompat.getInsetsController(window, window.decorView)
        controller.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        controller.hide(WindowInsetsCompat.Type.systemBars())
    }
}
