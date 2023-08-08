#include <LiquidCrystal.h> //Dołączenie bilbioteki
#include <string.h>
#define RED 9
#define GREEN 10
#define BLUE 11
#define DELAY 1
#define CUT 256
#define STARITNG 0

LiquidCrystal lcd(2, 3, 4, 5, 6, 7);


void setup() {
  // put your setup code here, to run once:
  lcd.begin(16, 2);
  lcd.setCursor(0, 0);
  Serial.begin(9600);
  Serial.setTimeout(10);
  pinMode(RED, OUTPUT);
  pinMode(GREEN, OUTPUT);
  pinMode(BLUE, OUTPUT);
            analogWrite(RED, 0);

            analogWrite(GREEN, 0);

            analogWrite(BLUE, 0);

}
  

void loop() {
  bool static is_on = false;
  int static counter = 0;
  String str;
  str = Serial.readString();
  if (str != "") {
    if (str[0] == '1') {
      if (str[1] == '1') {
        analogWrite(RED, 255);
        analogWrite(GREEN, 255);
        analogWrite(BLUE, 255);
        switch (counter) {
          case 0:
            for(int i = STARITNG; i<CUT; i++) {
              analogWrite(BLUE, i);
              delay(DELAY);
            }
            break;
          case 1:
            for(int i = STARITNG; i<CUT; i++) {
              analogWrite(GREEN, i);
              analogWrite(BLUE, i);
              delay(DELAY);
            }
            break;
          case 2:
            for(int i = STARITNG; i<CUT; i++) {
              analogWrite(GREEN, i);
              delay(DELAY);
            }
            break;
          case 3:
            for(int i = STARITNG; i<CUT; i++) {
              analogWrite(RED, i);
              analogWrite(GREEN, i);
              delay(DELAY);
            }
            break;
          case 4:
            for(int i = STARITNG; i<CUT; i++) {
              analogWrite(RED, i);
              delay(DELAY);
            }
            break;
          case 5:
            for(int i = STARITNG; i<CUT; i++) {
              analogWrite(RED, i);
              analogWrite(BLUE, i);
              delay(DELAY);
            }
            counter = -1;
            break;
        }
        counter++;
      } else {
        static bool div = false;
        switch (counter) {
          case 0:
            analogWrite(RED, 0);
            analogWrite(GREEN, 255);
            analogWrite(BLUE, 255);
            break;
          case 1:
            analogWrite(RED, 0);
            analogWrite(GREEN, 0);
            analogWrite(BLUE, 255);
            break;
          case 2:
            analogWrite(RED, 255);
            analogWrite(GREEN, 0);
            analogWrite(BLUE, 255);
            break;
          case 3:
            analogWrite(RED, 255);
            analogWrite(GREEN, 0);
            analogWrite(BLUE, 0);
            break;
          case 4:
            analogWrite(RED, 255);
            analogWrite(GREEN, 255);
            analogWrite(BLUE, 0);
            break;
          case 5:
            analogWrite(RED, 0);
            analogWrite(GREEN, 255);
            analogWrite(BLUE, 0);
            counter = -1;
            break;
        }
        if (div) {
          if (counter == 5) {
            counter = -1;
          }
          counter++;
        }
        div = !div;

      }
      
    } else {
      int i = str.indexOf("&");
      String title = str.substring(1, i);
      String artists = str.substring(i+1, str.length());
      lcd.clear();
      lcd.print(title);
      lcd.setCursor(0, 1);
      lcd.print(artists);
    }
  }
}
