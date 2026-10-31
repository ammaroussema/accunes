package com.accunes.ra

import android.content.Context
import android.content.SharedPreferences
import com.accunes.Emulator
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.net.HttpURLConnection
import java.net.URL
import org.json.JSONObject

class RetroAchievementsManager(context: Context) {

    private val prefs: SharedPreferences = context.getSharedPreferences("accunes_ra", Context.MODE_PRIVATE)

    var username: String?
        get() = prefs.getString("username", null)
        private set(value) = prefs.edit().putString("username", value).apply()

    var apiToken: String?
        get() = prefs.getString("token", null)
        private set(value) = prefs.edit().putString("token", value).apply()

    val isLoggedIn: Boolean
        get() = !username.isNullOrEmpty() && !apiToken.isNullOrEmpty()

    suspend fun login(user: String, pass: String): Boolean = withContext(Dispatchers.IO) {
        try {
            val url = URL("https://retroachievements.org/dorequest.php?r=login&u=$user&p=$pass")
            val conn = url.openConnection() as HttpURLConnection
            conn.connectTimeout = 8000
            conn.readTimeout = 8000
            val resp = conn.inputStream.bufferedReader().use { it.readText() }
            val json = JSONObject(resp)
            if (json.optBoolean("Success", false)) {
                username = json.optString("User", user)
                apiToken = json.optString("Token", "")
                true
            } else {
                false
            }
        } catch (_: Exception) {
            false
        }
    }

    fun logout() {
        prefs.edit().clear().apply()
    }
}
