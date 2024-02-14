!!!! You can modify ./language/cur.properties for localization. !!!



Translated with www.DeepL.com/Translator (free version)

---------- ---------- ---------- ------
Installation and use instructions

1, after opening if prompted JVM NOT FOUND, please download and install the latest JAVA runtime environment JRE
https://www.java.com/zh-CN/download/

2, the game please set to full-screen window (no border) mode or window mode to run

3、Because the JAVA form transparent implementation calls the system AERO transparent interface, in xp system and change to basic theme, classic theme of win7 system (that is, close AERO), will occur "transparent hover window affects the performance of all games" problem, please upgrade to WIN10 or switch to other than Please upgrade to WIN10 or switch to a theme other than "BASIC" or "Classic" (i.e. open AERO)

4、Win10.1 has a bug that the taskbar cannot be hidden, please use it with TaskBarHider and press CTRL+~ to hide the taskbar after opening.

5、If you find that the program resource consumption is too high, please increase the advanced settings - data frame delay (milliseconds), and enable the advanced settings - simplify the font stroke

---------- ---------- ---------- ------
Configuration notes

The following feature configuration needs to be opened in Notepad. /config/config.properties, manually modify the properties in the configuration

Artificial horizon size: modify attitudeIndicatorWidth and attitudeIndicatorHeight to adjust the size of the artificial horizon

The simplest HUD control label display: set disableHUDXXXLabel=true to turn off some labels of the simplest HUD
The simplest HUD interface level (pitch, roll and slide angle) display: set drawHUDAttitude=false to turn off drawing level information
Minimal HUD display Mach number: set hudMach=true to replace the display speed with Mach number

---------- ---------- ---------- ------
FM information description

- Critical speed: the stall speed and the maximum speed allowed by the structure respectively
- Three-axis rotational inertia: the smaller the rotational inertia, the more agile and lighter the maneuvering feel
- Spreading chord ratio: equal to the square of the wingspan length ÷ wing area. The larger the spar ratio, the lower the induced drag and the lower the maneuvering energy loss.
- Wingspan efficiency: Oswalds wingspan efficiency factor, the larger the wingspan efficiency, the smaller the induced drag, the lower the maneuvering energy loss.
- Induced drag factor: (1/(circumference x wing span efficiency x chord ratio)), the induced drag factor is linearly related to the induced drag.
- Induced drag acceleration factor: Using the induced drag factor divided by the aircraft's semi-fuel weight, it can be interpreted as the degree of change of induced drag on the aircraft's velocity vector at a certain speed.
- The higher the effective speed, the lower the rudder lock factor and the less likely to lock rudder at high speed.
- Main drag area factor C_d × S(Σ(FM component zero-lift drag coefficient × component area))::: approximate drag coefficient in level flight (smaller than the actual value, because there is still a certain negative angle of attack in level flight, there is induced drag)
- Main lift area factor load Cl_max × S/m, the algorithm is Σ(main component maximum lift coefficient × its component area) ÷ semi-fuel takeoff weight, due to the large difference in lift coefficients between Anton Star models, the main lift area factor load is more reflective of the aircraft's instantaneous hovering performance than the wing load
- Main drag area acceleration factor: Using the main drag area factor divided by the semi-fuel weight, it can be interpreted as the degree of change of zero-lift drag on the aircraft speed vector during unpowered gliding. The smaller the value, the longer it takes to converge from the current airspeed to the maximum level flight speed, which means that the (unpowered) dive/glide speed storage capability is better.
- Critical angle of attack: The trend of the lift curve Cl(AoA) of an aerodynamic component with the angle of attack is roughly a straight line with a fixed slope - then a parabolic decline, and the critical angle of attack is usually the angle of attack when the maximum lift is achieved, beyond the critical angle of attack, the lift of the aerodynamic component will be reduced.

---------- ---------- ---------- ------
Minimal HUD Description

The Minimal HUD focuses on the combination of simple numbers and intuitive graphical elements to quickly present flight status information to the player in order to optimize the speed of the OODA loop.
The meanings of the main elements are shown below.
I-Shows airspeed km/h, you can change the "hudMach=true" in the config to show Mach number;
H-Altitude m;
S-Residual power SEP, unit is m/s;
L-Fuel estimation time, in minutes;
G-Normal overload;
F-Flap percentage;
GEAR-Landing gear;
BRK-Reduction plate;
W-Variable wing swept back angle
α-Angle of Attack
Disc pointer and numbers - heading; 
Red line inside the disc pointer-engine heat resistance warning bar, turning full means engine failure is imminent;
The first green vertical bar from the left - throttle;
The first green horizontal bar from the left - available angle of attack, available angle of attack to 0 will stall.

Minimal HUD interface using equal-width font, can be configured to modify the configuration file "MonoNumFont=Consolas" to replace the current Consolas font

---------- ---------- ---------- ------
Voice alarm description

The voice alarm plays when the trigger conditions are met, the trigger conditions and voice prompts are as follows.

Exceeding the critical angle of attack of this model wing - "Angle of attack exceeded"
Approaching landing gear blow-off speed (25km/h) - "Landing gear overspeed"
Approaching flap blow speed (25km/h) - "Flap overspeed" 
Fuel below 10% - "Fuel low"
Fuel 0% - "Fuel depleted"
Gauge speed close to permissible speed (40km/h) - "Gauge speed high"
Mach close to permissible Mach number (Mach 0.05) - "Mach number large"
Engine thermal time less than 300 seconds - "Engine over temperature"
Stopping due to engine damage or high oil to gas mixture ratio (manifested by oil pressure less than the given throttle for more than 10 seconds) - "Low oil pressure"
Black engine (oil pressure drops to 0 after engine damage) - "Engine failure"
Low altitude with a large rate of descent (one tenth of the current altitude) - "Altitude"
Overload number close to maximum wing overload (1G) - "Overload overload"
Approaching stall speed - "Stall"
Landing gear open and descent rate greater than 8m/s - "Descent rate high"
Negative G and fuel cut - "Engine failure"
RPM percentage behind throttle by 30% - "RPM low"
RPM exceeds max RPM by 5% - "RPM high"
Radio altitude less than one-tenth of descent rate - "Terrain"

Customizable voice alerts:
You can replace the corresponding .wav file in the voice directory by yourself to achieve a custom alarm voice
Note that this program only supports audio files in wav format, please make sure to convert audio files in other formats.


---------- ---------- ---------- ------
Custom Text Description

Modify . /language/cur.properties to replace the text displayed in this program.
