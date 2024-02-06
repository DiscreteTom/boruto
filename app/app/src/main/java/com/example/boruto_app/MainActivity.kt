package com.example.boruto_app

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import android.util.Log
import android.widget.Toast
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.camera.core.*
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.core.content.ContextCompat
import androidx.navigation.findNavController
import androidx.navigation.ui.AppBarConfiguration
import androidx.navigation.ui.setupActionBarWithNavController
import androidx.navigation.ui.setupWithNavController
import com.example.boruto_app.databinding.ActivityMainBinding
import com.google.android.material.bottomnavigation.BottomNavigationView
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.face.FaceDetection
import com.google.mlkit.vision.face.FaceDetectorOptions
import io.ktor.client.*
import io.ktor.client.engine.cio.*
import io.ktor.client.plugins.websocket.*
import io.ktor.http.*
import io.ktor.websocket.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.runBlocking
import java.util.*
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

class MainActivity : AppCompatActivity() {
    private lateinit var viewBinding: ActivityMainBinding
    private lateinit var cameraExecutor: ExecutorService
    private lateinit var websocketExecutor: ExecutorService
    private val channel = Channel<Float>()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        viewBinding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(viewBinding.root)

        // nav
        val navView: BottomNavigationView = viewBinding.navView
        val navController = findNavController(R.id.nav_host_fragment_activity_main)
        // Passing each menu ID as a set of Ids because each
        // menu should be considered as top level destinations.
        val appBarConfiguration = AppBarConfiguration(
            setOf(
                R.id.navigation_home, R.id.navigation_dashboard, R.id.navigation_notifications
            )
        )
        setupActionBarWithNavController(navController, appBarConfiguration)
        navView.setupWithNavController(navController)

        cameraExecutor = Executors.newSingleThreadExecutor()
        websocketExecutor = Executors.newSingleThreadExecutor()

        // Request camera permissions
        if (allPermissionsGranted()) {
            startCamera()
        } else {
            requestPermissions()
        }

        // Set up the listeners for take photo and video capture buttons
//        viewBinding.imageCaptureButton.setOnClickListener { takePhoto() }
//        viewBinding.videoCaptureButton.setOnClickListener { captureVideo() }
    }

    private fun requestPermissions() {
        activityResultLauncher.launch(REQUIRED_PERMISSIONS)
    }
    private val activityResultLauncher = registerForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { permissions ->
        // Handle Permission granted/rejected
        var permissionGranted = true
        permissions.entries.forEach {
            if (it.key in REQUIRED_PERMISSIONS && !it.value) {
                Toast.makeText(
                    baseContext, "Permission request denied for $it.value", Toast.LENGTH_SHORT
                ).show()
                permissionGranted = false
            }
        }
        if (!permissionGranted) {
            Toast.makeText(
                baseContext, "Permission request denied", Toast.LENGTH_SHORT
            ).show()
        } else {
            startCamera()
        }
    }
    private fun allPermissionsGranted() = REQUIRED_PERMISSIONS.all {
        ContextCompat.checkSelfPermission(
            baseContext, it
        ) == PackageManager.PERMISSION_GRANTED
    }

    private fun startCamera() {
        val cameraProviderFuture = ProcessCameraProvider.getInstance(this)

        val client = HttpClient(CIO) {
            install(WebSockets) {}
        }
        websocketExecutor.execute {
            runBlocking {
                try {
                    client.webSocket(method = HttpMethod.Get, host = "192.168.1.6", port = 9002) {
                        // debounce with history
                        val history = mutableListOf<Float>()
                        while(true) {
                            val ey = channel.receive()
                            history.add(ey * 50)
                            // keep the history size to a fixed value and calculate the mean value to smooth the move
                            while (history.size > 8) history.removeAt(0)
                            val x = history.average().toInt()
                            send(Frame.Text("{\"type\":\"update\",\"x\":$x,\"y\":0}"))
                        }
                    }
                } catch (e: Exception) {
                    Toast.makeText(
                        baseContext, e.toString(), Toast.LENGTH_SHORT
                    ).show()
                }
            }
            client.close()
        }

        cameraProviderFuture.addListener({
            // Used to bind the lifecycle of cameras to the lifecycle owner
            val cameraProvider: ProcessCameraProvider = cameraProviderFuture.get()

            // Preview
            val preview = Preview.Builder().build().also {
                    it.setSurfaceProvider(viewBinding.viewFinder.surfaceProvider)
                }

            // Select front camera as a default
            val cameraSelector = CameraSelector.DEFAULT_FRONT_CAMERA

            val imageAnalyzer = ImageAnalysis.Builder().build().also {
                it.setAnalyzer(cameraExecutor, FaceTrackingAnalyzer(viewBinding, channel))
            }

            try {
                // Unbind use cases before rebinding
                cameraProvider.unbindAll()

                // Bind use cases to camera
                cameraProvider.bindToLifecycle(
                    this, cameraSelector, preview, imageAnalyzer
                )
            } catch (exc: Exception) {
                Log.e(TAG, "Use case binding failed", exc)
            }
        }, ContextCompat.getMainExecutor(this))
    }

    override fun onDestroy() {
        super.onDestroy()
        cameraExecutor.shutdown()
        websocketExecutor.shutdown()
    }

    companion object {
        private const val TAG = "CameraXApp"
        private const val FILENAME_FORMAT = "yyyy-MM-dd-HH-mm-ss-SSS"
        private val REQUIRED_PERMISSIONS = mutableListOf(
            Manifest.permission.CAMERA
        ).toTypedArray()
    }


    private class FaceTrackingAnalyzer(val viewBinding: ActivityMainBinding, val channel: Channel<Float>) : ImageAnalysis.Analyzer {
        val options = FaceDetectorOptions.Builder().build()
        val detector = FaceDetection.getClient(options)
        var last:Long = 0

        @ExperimentalGetImage
        override fun analyze(imageProxy: ImageProxy) {
            val mediaImage = imageProxy.image
            if (mediaImage != null) {
                val image =
                    InputImage.fromMediaImage(mediaImage, imageProxy.imageInfo.rotationDegrees)
                val now = System.currentTimeMillis()
                val interval = now - last

                detector.process(image).addOnSuccessListener { faces ->
                        // Task completed successfully
                        // ...
                        if (faces.size > 0){
                            val ex = faces[0].headEulerAngleX
                            val ey = faces[0].headEulerAngleY
                            viewBinding.euler.text = "Euler X: $ex\nEuler Y: $ey"
                            val latency = System.currentTimeMillis() - now
                            last = now
                            viewBinding.latency.text = "Process Latency: ${latency}ms\nFrame Interval: ${interval}ms"
                            channel.trySend(ey)
                        }
                    }.addOnFailureListener { _ ->
                        // Task failed with an exception
                        // ...
                    }.addOnCompleteListener {
                        // see https://developers.google.com/ml-kit/vision/face-detection/android#4.-process-the-image
                        imageProxy.close()
                    }
            }
        }
    }
}