package vitapmate.features.biometric

import com.composables.icons.lucide.FingerprintPattern
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Feature

/** Biometric history: one day's face and fingerprint punches, newest first. */
val biometricFeature = Feature("biometric", "Biometric", Lucide.FingerprintPattern) { BiometricScreen() }
