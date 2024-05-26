#include "Freenove_WS2812_Lib_for_ESP32.h"

#define LEDS_COUNT  5
#define LEDS_PIN  10
#define CHANNEL   0

Freenove_ESP32_WS2812 strip = Freenove_ESP32_WS2812(LEDS_COUNT, LEDS_PIN, CHANNEL);

#define uS_TO_mS_FACTOR 1000ULL  /* Conversion factor for micro seconds to seconds */
#define TIME_TO_SLEEP  50        /* Time ESP32 will go to sleep (in seconds) */

RTC_DATA_ATTR int bootCount = 0;

void setup(){
  //Serial.begin(115200);

  strip.begin();
  //delay(10); //Take some time to open up the Serial Monitor
  strip.setBrightness(50);
  for (int i = 0; i < LEDS_COUNT; i++) {
    strip.setLedColorData(i, strip.Wheel((i * 256 / LEDS_COUNT + bootCount*2) & 255));
    //strip.setLedColorData(i, 255,255,255);
  }

  strip.show();
  delay(2);
  //Increment boot number and print it every reboot
  ++bootCount;
  //Serial.println("Boot number: " + String(bootCount));

  /*
  First we configure the wake up source
  We set our ESP32 to wake up every 5 seconds
  */
  esp_sleep_enable_timer_wakeup(TIME_TO_SLEEP * uS_TO_mS_FACTOR);
  //Serial.println("Setup ESP32 to sleep for every " + String(TIME_TO_SLEEP) +  " Seconds");
  
  //Serial.println("sleep now");
  //Serial.flush(); 
  esp_deep_sleep_start();
  // uC tired, uC sleepy
}

void loop(){
  //This is not going to be called
}
