# timing.py OUT CROP POINTER_IN POINTER_OUT PROBE: records the display at 60 fps while the pointer moves in and out,
# and prints when the PROBE region (relative to CROP) changes, in frames after each move. Example:
#   python3 -I timing.py out 560x70+0+60 200,120 540,120 20x10+466+18
import subprocess, sys, time, os, glob
out, crop, pin, pout, probe = sys.argv[1:6]
w,rest = crop.split('x'); h,x,y = rest.split('+'); x,y=int(x),int(y)
os.makedirs(out, exist_ok=True)
for f in glob.glob(out+'/f*.png'): os.remove(f)
env = dict(os.environ, DISPLAY=':99')
mv=lambda p: subprocess.run(['xdotool','mousemove',*p.split(',')], env=env)
mv(pout); time.sleep(1.5)
rec = subprocess.Popen(['ffmpeg','-loglevel','error','-f','x11grab','-draw_mouse','1','-framerate','60','-video_size',f'{w}x{h}','-i',f':99+{x},{y}','-t','3.2','-y',out+'/v.mkv'], env=env)
time.sleep(0.6); mv(pin); time.sleep(1.4); mv(pout); rec.wait()
subprocess.run(['ffmpeg','-loglevel','error','-i',out+'/v.mkv',out+'/f%04d.png'])
fs=sorted(glob.glob(out+'/f*.png'))
def at(f,region,fmt): return subprocess.run(['convert',f,'-crop',region,'+repage','-colorspace','gray','-format',fmt,'info:'],capture_output=True,text=True).stdout
px,py=[int(v) for v in pin.split(',')]; ox,oy=[int(v) for v in pout.split(',')]
cur_in=f'12x12+{px-x}+{py-y}'; cur_out=f'12x12+{ox-x}+{oy-y}'
sig=[]
for f in fs:
    sig.append((at(f,cur_in,'%#'),at(f,cur_out,'%#'),float(at(f,probe,'%[fx:maxima]'))))
t_in=next(i for i in range(1,len(sig)) if sig[i][0]!=sig[i-1][0])
t_out=next(i for i in range(t_in+5,len(sig)) if sig[i][1]!=sig[i-1][1])
vals=[s[2] for s in sig]; lo,hi=min(vals),max(vals)
n=lambda v:(v-lo)/(hi-lo+1e-9)
def ramp(t0):
    s=[(i-t0, round(n(vals[i]),2)) for i in range(t0-1,min(len(vals),t0+40))]
    first=next((k for k,v in s if 0.02<v<0.98 or (k>=0 and abs(v-s[0][1])>0.02)),None)
    return s
for name,t0 in (('IN',t_in),('OUT',t_out)):
    s=ramp(t0); changes=[(k,v) for (k,v),(k2,v2) in zip(s[1:],s) if v!=v2]
    print(name,'frames(16.7ms):',changes)
