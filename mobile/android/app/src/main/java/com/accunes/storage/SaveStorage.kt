package com.accunes.storage

import android.content.Context
import com.accunes.Emulator
import java.io.File

class SaveStorage(context: Context) {

    private val savesDir = File(context.filesDir, "saves").apply { mkdirs() }

    fun saveSram(emulator: Emulator, romHash: String) {
        val sram = emulator.getSram() ?: return
        val file = File(savesDir, "$romHash.sav")
        file.writeBytes(sram)
    }

    fun loadSram(emulator: Emulator, romHash: String) {
        val file = File(savesDir, "$romHash.sav")
        if (file.exists() && file.length() > 0) {
            emulator.setSram(file.readBytes())
        }
    }

    fun saveStateSlot(emulator: Emulator, romHash: String, slot: Int): Boolean {
        val state = emulator.saveState() ?: return false
        val file = File(savesDir, "${romHash}_slot$slot.state")
        file.writeBytes(state)
        return true
    }

    fun loadStateSlot(emulator: Emulator, romHash: String, slot: Int): Boolean {
        val file = File(savesDir, "${romHash}_slot$slot.state")
        if (!file.exists()) return false
        return emulator.loadState(file.readBytes())
    }

    fun hasStateSlot(romHash: String, slot: Int): Boolean {
        return File(savesDir, "${romHash}_slot$slot.state").exists()
    }
}
