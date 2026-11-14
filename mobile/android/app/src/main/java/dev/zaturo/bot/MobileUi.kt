package dev.zaturo.bot

import android.app.Activity
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.RippleDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.TextView
import kotlin.math.roundToInt

/** Native controls only; their parent owns layout and application behavior. */
internal class MobileUi(private val activity: Activity) {
    fun dp(value: Int): Int = (value * activity.resources.displayMetrics.density).roundToInt()

    fun text(value: String, size: Int, color: Int = WHITE, bold: Boolean = false): TextView =
        TextView(activity).apply {
            text = value
            textSize = size.toFloat()
            setTextColor(color)
            includeFontPadding = false
            typeface = font(bold)
        }

    fun rounded(fill: Int = SURFACE, radius: Int = 16, stroke: Int = STROKE): GradientDrawable =
        GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(fill)
            cornerRadius = dp(radius).toFloat()
            setStroke(dp(1), stroke)
        }

    fun button(
        label: String,
        icon: Int = 0,
        primary: Boolean = false,
        danger: Boolean = false,
        compact: Boolean = false,
        action: () -> Unit,
    ): Button = Button(activity).apply {
        val emphasized = primary && !danger
        val foreground = when {
            danger -> DANGER_TEXT
            emphasized -> BG
            else -> WHITE
        }
        val colors = stateColors(foreground, emphasized)
        text = label
        contentDescription = label
        isAllCaps = false
        textSize = if (compact) 12f else 14f
        includeFontPadding = false
        typeface = font(true)
        gravity = Gravity.CENTER
        maxLines = 1
        ellipsize = TextUtils.TruncateAt.END
        setTextColor(colors)
        minWidth = dp(48)
        minHeight = dp(48)
        minimumWidth = dp(48)
        minimumHeight = dp(48)
        setPadding(dp(if (compact) 12 else 16), dp(if (compact) 10 else 12),
            dp(if (compact) 12 else 16), dp(if (compact) 10 else 12))
        elevation = 0f
        translationZ = 0f
        stateListAnimator = null
        backgroundTintList = null
        background = ripple(if (emphasized) YELLOW else SURFACE,
            if (emphasized) YELLOW else STROKE, foreground)
        if (icon != 0) {
            val drawable = activity.getDrawable(icon)?.mutate()?.apply {
                setTintList(colors)
                setBounds(0, 0, dp(24), dp(24))
            }
            setCompoundDrawablesRelative(drawable, null, null, null)
            compoundDrawablePadding = dp(8)
        }
        setOnClickListener { action() }
    }

    fun iconButton(label: String, icon: Int, primary: Boolean = false, action: () -> Unit): ImageButton =
        ImageButton(activity).apply {
            val foreground = if (primary) BG else WHITE
            contentDescription = label
            setImageResource(icon)
            imageTintList = stateColors(foreground, primary)
            scaleType = ImageView.ScaleType.CENTER_INSIDE
            minimumWidth = dp(48)
            minimumHeight = dp(48)
            setPadding(dp(12), dp(12), dp(12), dp(12))
            elevation = 0f
            translationZ = 0f
            stateListAnimator = null
            backgroundTintList = null
            background = ripple(if (primary) YELLOW else SURFACE,
                if (primary) YELLOW else STROKE, foreground)
            setOnClickListener { action() }
        }

    fun icon(resource: Int, color: Int = WHITE, size: Int = 24): ImageView =
        ImageView(activity).apply {
            setImageResource(resource)
            imageTintList = ColorStateList.valueOf(color)
            scaleType = ImageView.ScaleType.FIT_CENTER
            adjustViewBounds = true
            minimumWidth = dp(size)
            minimumHeight = dp(size)
            maxWidth = dp(size)
            maxHeight = dp(size)
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        }

    fun edit(hint: String): EditText = EditText(activity).apply {
        isSaveEnabled = false
        this.hint = hint
        textSize = 14f
        includeFontPadding = false
        typeface = font(false)
        setTextColor(WHITE)
        setHintTextColor(MUTED)
        minHeight = dp(48)
        minimumHeight = dp(48)
        setPadding(dp(12), dp(10), dp(12), dp(10))
        elevation = 0f
        stateListAnimator = null
        backgroundTintList = null
        background = rounded()
        highlightColor = withAlpha(YELLOW, 64)
    }

    private fun font(emphasized: Boolean): Typeface =
        Typeface.create(if (emphasized) "sans-serif-medium" else "sans-serif", Typeface.NORMAL)

    private fun stateColors(foreground: Int, primary: Boolean): ColorStateList = ColorStateList(
        arrayOf(intArrayOf(android.R.attr.state_enabled), intArrayOf()),
        intArrayOf(foreground, if (primary) withAlpha(BG, 160) else MUTED),
    )

    private fun ripple(fill: Int, stroke: Int, ink: Int): RippleDrawable = RippleDrawable(
        ColorStateList.valueOf(withAlpha(ink, 32)),
        rounded(fill, 16, stroke),
        rounded(WHITE, 16, WHITE),
    )

    private fun withAlpha(color: Int, alpha: Int): Int =
        Color.argb(alpha, Color.red(color), Color.green(color), Color.blue(color))

    companion object {
        const val BG: Int = 0xff0b0b0d.toInt()
        const val SURFACE: Int = 0xff17171b.toInt()
        const val STROKE: Int = 0xff303036.toInt()
        const val MUTED: Int = 0xffa6a6b0.toInt()
        const val YELLOW: Int = 0xffe5b900.toInt()
        const val WHITE: Int = -1
        const val USER_BUBBLE: Int = YELLOW
        const val USER_TEXT: Int = BG
        private const val DANGER_TEXT: Int = 0xffefa6a6.toInt()
    }
}
