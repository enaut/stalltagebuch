package dev.dioxus.main

import android.Manifest
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Bundle
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import java.io.File
import java.text.SimpleDateFormat
import java.util.*

/**
 * MainActivity für Stalltagebuch
 * Erweitert WryActivity (Dioxus) mit Camera & Gallery Integration
 */
class MainActivity : WryActivity() {
    
    companion object {
        private const val CAMERA_PERMISSION_CODE = 1001
        private const val STORAGE_PERMISSION_CODE = 1002
        
        // Singleton-Referenz auf die Activity
        @Volatile
        private var instance: MainActivity? = null
        
        @JvmStatic
        fun getInstance(): MainActivity? = instance
        
        // Static variables für JNI-Zugriff
        @Volatile
        private var currentPhotoPath: String? = null
        @Volatile
        private var currentPhotoPaths: String? = null // Newline-separated paths for multi-select
        
        @Volatile
        private var lastError: String? = null
        
        @JvmStatic
        fun getLastPhotoPath(): String? = currentPhotoPath
        
        @JvmStatic
        fun getLastError(): String? = lastError
        
        @JvmStatic
        fun getLastPhotoPaths(): String? = currentPhotoPaths
        
        @JvmStatic
        fun clearLastError() {
            lastError = null
        }
    }
    
    // ActivityResultLauncher für Gallery-Auswahl (single)
    private lateinit var pickImageLauncher: ActivityResultLauncher<String>
    // ActivityResultLauncher für Gallery-Auswahl (multiple)
    private lateinit var pickImagesLauncher: ActivityResultLauncher<String>
    
    // ActivityResultLauncher für Kamera
    private lateinit var takePictureLauncher: ActivityResultLauncher<Uri>
    
    // Temporäre URI für Kamera-Foto
    private var photoUri: Uri? = null
    
    // Pending action nach Permission-Grant
    private var pendingAction: (() -> Unit)? = null
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        instance = this
        
        // Register Gallery-Picker
        pickImageLauncher = registerForActivityResult(
            ActivityResultContracts.GetContent()
        ) { uri: Uri? ->
            if (uri != null) {
                // Kopiere ausgewähltes Bild in internen Speicher
                try {
                    val photoFile = createImageFile()
                    contentResolver.openInputStream(uri)?.use { input ->
                        photoFile.outputStream().use { output ->
                            input.copyTo(output)
                        }
                    }
                    currentPhotoPath = photoFile.absolutePath
                    currentPhotoPaths = null
                    lastError = null
                } catch (e: Exception) {
                    lastError = "Fehler beim Kopieren des Bildes: ${e.message}"
                    currentPhotoPath = null
                    currentPhotoPaths = null
                }
            } else {
                lastError = "Keine Datei ausgewählt"
                currentPhotoPath = null
                currentPhotoPaths = null
            }
        }
        
        // Register Gallery-Picker (multiple)
        pickImagesLauncher = registerForActivityResult(
            ActivityResultContracts.GetMultipleContents()
        ) { uris: List<Uri> ->
            if (!uris.isNullOrEmpty()) {
                try {
                    val paths = mutableListOf<String>()
                    for (uri in uris) {
                        val photoFile = createImageFile()
                        contentResolver.openInputStream(uri)?.use { input ->
                            photoFile.outputStream().use { output ->
                                input.copyTo(output)
                            }
                        }
                        paths.add(photoFile.absolutePath)
                    }
                    currentPhotoPaths = paths.joinToString("\n")
                    currentPhotoPath = null
                    lastError = null
                } catch (e: Exception) {
                    lastError = "Fehler beim Kopieren der Bilder: ${e.message}"
                    currentPhotoPaths = null
                    currentPhotoPath = null
                }
            } else {
                lastError = "Keine Dateien ausgewählt"
                currentPhotoPaths = null
                currentPhotoPath = null
            }
        }
        
        // Register Kamera
        takePictureLauncher = registerForActivityResult(
            ActivityResultContracts.TakePicture()
        ) { success: Boolean ->
            if (success && photoUri != null) {
                // Foto wurde erfolgreich aufgenommen
                // photoUri.path gibt uns den richtigen Pfad zur Datei
                val path = photoUri?.let { uri ->
                    // Extrahiere den tatsächlichen Dateipfad aus der Content URI
                    val file = File(getExternalFilesDir("photos"), uri.lastPathSegment ?: "")
                    file.absolutePath
                }
                currentPhotoPath = path
                currentPhotoPaths = null
                lastError = null
            } else {
                lastError = "Foto-Aufnahme abgebrochen oder fehlgeschlagen"
                currentPhotoPath = null
                currentPhotoPaths = null
            }
        }
    }
    
    override fun onDestroy() {
        super.onDestroy()
        if (instance == this) {
            instance = null
        }
    }
    
    /**
     * Prüfe ob Camera-Permission vorhanden ist
     */
    fun hasCameraPermission(): Boolean {
        return ContextCompat.checkSelfPermission(
            this,
            Manifest.permission.CAMERA
        ) == PackageManager.PERMISSION_GRANTED
    }
    
    /**
     * Prüfe ob Storage-Permission vorhanden ist
     */
    fun hasStoragePermission(): Boolean {
        // Ab Android 13 (API 33) gibt es READ_MEDIA_IMAGES statt READ_EXTERNAL_STORAGE
        return if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.TIRAMISU) {
            ContextCompat.checkSelfPermission(
                this,
                Manifest.permission.READ_MEDIA_IMAGES
            ) == PackageManager.PERMISSION_GRANTED
        } else {
            ContextCompat.checkSelfPermission(
                this,
                Manifest.permission.READ_EXTERNAL_STORAGE
            ) == PackageManager.PERMISSION_GRANTED
        }
    }
    
    /**
     * Fordere Camera-Permission an
     */
    fun requestCameraPermission() {
        ActivityCompat.requestPermissions(
            this,
            arrayOf(Manifest.permission.CAMERA),
            CAMERA_PERMISSION_CODE
        )
    }
    
    /**
     * Fordere Storage-Permission an
     */
    fun requestStoragePermission() {
        val permissions = if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.TIRAMISU) {
            arrayOf(Manifest.permission.READ_MEDIA_IMAGES)
        } else {
            arrayOf(
                Manifest.permission.READ_EXTERNAL_STORAGE,
                Manifest.permission.WRITE_EXTERNAL_STORAGE
            )
        }
        
        ActivityCompat.requestPermissions(
            this,
            permissions,
            STORAGE_PERMISSION_CODE
        )
    }
    
    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        
        when (requestCode) {
            CAMERA_PERMISSION_CODE -> {
                if (grantResults.isNotEmpty() && grantResults[0] == PackageManager.PERMISSION_GRANTED) {
                    // Permission granted, execute pending action
                    pendingAction?.invoke()
                    pendingAction = null
                } else {
                    lastError = "Kamera-Berechtigung verweigert"
                }
            }
            STORAGE_PERMISSION_CODE -> {
                if (grantResults.isNotEmpty() && grantResults[0] == PackageManager.PERMISSION_GRANTED) {
                    // Permission granted, execute pending action
                    pendingAction?.invoke()
                    pendingAction = null
                } else {
                    lastError = "Speicher-Berechtigung verweigert"
                }
            }
        }
    }
    
    /**
     * Öffne Gallery für Bildauswahl
     * Von Rust via JNI aufrufbar
     */
    fun launchImagePicker() {
        try {
            currentPhotoPath = null
            currentPhotoPaths = null
            lastError = null
            
            if (!hasStoragePermission()) {
                pendingAction = { launchImagePickerInternal() }
                requestStoragePermission()
            } else {
                launchImagePickerInternal()
            }
        } catch (e: Exception) {
            lastError = "Fehler beim Öffnen der Gallery: ${e.message}"
        }
    }
    
    private fun launchImagePickerInternal() {
        pickImageLauncher.launch("image/*")
    }
    
    /**
     * Öffne Gallery für Mehrfachauswahl
     * Von Rust via JNI aufrufbar
     */
    fun launchImagePickerMulti() {
        try {
            currentPhotoPath = null
            currentPhotoPaths = null
            lastError = null
            
            if (!hasStoragePermission()) {
                pendingAction = { launchImagePickerMultiInternal() }
                requestStoragePermission()
            } else {
                launchImagePickerMultiInternal()
            }
        } catch (e: Exception) {
            lastError = "Fehler beim Öffnen der Gallery (multi): ${e.message}"
        }
    }
    
    private fun launchImagePickerMultiInternal() {
        pickImagesLauncher.launch("image/*")
    }
    
    /**
     * Öffne Kamera für Foto-Aufnahme
     * Von Rust via JNI aufrufbar
     */
    fun launchCamera() {
        try {
            currentPhotoPath = null
            lastError = null
            
            if (!hasCameraPermission()) {
                pendingAction = { launchCameraInternal() }
                requestCameraPermission()
            } else {
                launchCameraInternal()
            }
        } catch (e: Exception) {
            lastError = "Fehler beim Öffnen der Kamera: ${e.message}"
        }
    }
    
    private fun launchCameraInternal() {
        try {
            // Erstelle temporäre Datei für Foto
            val photoFile = createImageFile()
            
            // Erstelle URI mit FileProvider
            photoUri = FileProvider.getUriForFile(
                this,
                "${packageName}.fileprovider",
                photoFile
            )
            
            // Starte Kamera mit URI
            takePictureLauncher.launch(photoUri)
            
        } catch (e: Exception) {
            lastError = "Fehler beim Starten der Kamera: ${e.message}"
        }
    }
    
    /**
     * Erstelle eindeutige Datei für Foto
     */
    private fun createImageFile(): File {
        val timestamp = SimpleDateFormat("yyyyMMdd_HHmmss", Locale.getDefault()).format(Date())
        val storageDir = getExternalFilesDir("photos") ?: filesDir
        
        if (!storageDir.exists()) {
            storageDir.mkdirs()
        }
        
        return File.createTempFile(
            "WACHTEL_${timestamp}_",
            ".jpg",
            storageDir
        )
    }
}
