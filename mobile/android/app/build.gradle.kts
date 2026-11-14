plugins { id("com.android.application") }
android {
    namespace="dev.zaturo.bot"
    compileSdk=36
    defaultConfig {applicationId="dev.zaturo.bot";minSdk=26;targetSdk=36;versionCode=2;versionName="0.4.1"}
    compileOptions {sourceCompatibility=JavaVersion.VERSION_17;targetCompatibility=JavaVersion.VERSION_17}
    buildTypes {release {isMinifyEnabled=false}}
    packaging {resources.excludes += setOf("META-INF/versions/**","META-INF/LICENSE*","META-INF/NOTICE*","META-INF/DEPENDENCIES")}
    testOptions {unitTests.isReturnDefaultValues=true}
}
dependencies {
    implementation("com.hierynomus:sshj:0.41.2")
    implementation("org.bouncycastle:bcprov-jdk18on:1.87")
    implementation("org.bouncycastle:bcpkix-jdk18on:1.87")
    implementation("org.slf4j:slf4j-nop:2.0.20")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.json:json:20260814")
}
