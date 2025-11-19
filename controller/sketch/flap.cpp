#include "Arduino.h"

extern "C" {
    void flap(){
        long falling[3]={0};
        long rising[3]={0};
        while(true){
            uint32_t start = micros();
            int count=100000;
            for(int i=0;i<count;i++){
                digitalWrite(13, false);
                falling[0]+=analogRead(14);
                falling[1]+=analogRead(14);
                falling[2]+=analogRead(14);
                digitalWrite(13, true);
                rising[0]+=analogRead(14);
                rising[1]+=analogRead(14);
                rising[2]+=analogRead(14);
            }
            uint32_t end=micros();
            uint32_t ms=(end-start)/count/2;
            Serial.print("ms = ");
            Serial.println(ms);
            Serial.print("falling ");
            Serial.print(falling[0]-falling[2]);
            Serial.print(" ");
            Serial.print(falling[1]-falling[2]);
            Serial.println();
            Serial.print("rising ");
            Serial.print(rising[2]-rising[0]);
            Serial.print(" ");
            Serial.print(rising[2]-rising[1]);
            Serial.println();
            delay(100);
        }
    }
}