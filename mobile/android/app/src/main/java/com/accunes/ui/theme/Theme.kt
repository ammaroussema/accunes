package com.accunes.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable

private val DarkColorScheme = darkColorScheme(
    primary = NesRed,
    secondary = NesLightGray,
    background = NesDark,
    surface = NesGray,
    onPrimary = NesWhite,
    onSecondary = NesWhite,
    onBackground = NesWhite,
    onSurface = NesWhite
)

@Composable
fun AccuNESTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = DarkColorScheme,
        typography = Typography,
        content = content
    )
}
