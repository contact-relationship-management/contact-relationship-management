package io.github.contactrelationshipmanagement.mobile

import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch

/** The one activity. It also receives the sign-in redirect (`launchMode="singleTask"`, MOB-4). */
class MainActivity : ComponentActivity() {
    private val model: CrmViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // MOB-15: no screenshot, screen recording or recent-apps thumbnail of a record screen.
        window.setFlags(WindowManager.LayoutParams.FLAG_SECURE, WindowManager.LayoutParams.FLAG_SECURE)
        enableEdgeToEdge()
        lifecycleScope.launch {
            model.browserRequests.collect { url ->
                try {
                    startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addCategory(Intent.CATEGORY_BROWSABLE))
                } catch (_: ActivityNotFoundException) {
                    model.noBrowser()
                }
            }
        }
        handle(intent)
        setContent { CrmApp(model) }
    }

    override fun onResume() {
        super.onResume()
        // A deployer may change or remove the managed configuration while the app is open (MOB-14).
        model.applyConfiguration()
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handle(intent)
    }

    private fun handle(intent: Intent?) {
        val data = intent?.data ?: return
        if (intent.action == Intent.ACTION_VIEW) {
            model.completeSignIn(data.toString())
            // The code is used once; do not let a configuration change hand it over again.
            intent.data = null
        }
    }
}
