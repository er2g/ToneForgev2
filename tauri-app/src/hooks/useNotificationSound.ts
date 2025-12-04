import { useCallback, useRef } from "react";

type SoundType = "success" | "error" | "notification" | "click";

// Base64 encoded short beep sounds (to avoid external files)
const SOUNDS: Record<SoundType, string> = {
  // Short success chime (C major chord)
  success: "data:audio/wav;base64,UklGRl4BAABXQVZFZm10IBAAAAABAAEARKwAAIhYAQACABAAZGF0YToBAAB//3v/d/9z/2//a/9n/2P/X/9b/1f/U/9P/0v/R/9D/z//O/83/zP/L/8r/yf/I/8f/xv/F/8T/w//C/8H/wP/AP/8/vj+9P7w/uz+6P7k/uD+3P7Y/tT+0P7M/sj+xP7A/rz+uP60/rD+rP6o/qT+oP6c/pj+lP6Q/oz+iP6E/oD+fP54/nT+cP5s/mj+ZP5g/lz+WP5U/lD+TP5I/kT+QP48/jj+NP4w/iz+KP4k/iD+HP4Y/hT+EP4M/gj+BP4A/vz99/3z/e/96/3n/eP94/3f/dv91/3T/dP90/3T/dP91/3X/dv93/3j/ef96/3v/fP99/37/f/+A/4H/gv+D/4T/hf+G/4f/iP+J/4r/i/+M/43/jv+P/5D/kf+S/5P/lP+V/5b/l/+Y/5n/mv+b/5z/nf+e/5//oP+h/6L/o/+k/6X/pv+n/6j/qf+q/6v/rP+t/67/r/+w/7H/sv+z/7T/tf+2/7f/uP+5/7r/u/+8/73/vv+//8D/wf/C/8P/xP/F/8b/x//I/8n/yv/L/8z/zf/O/8//0P/R/9L/0//U/9X/1v/X/9j/2f/a/9v/3P/d/97/3//g/+H/4v/j/+T/5f/m/+f/6P/p/+r/6//s/+3/7v/v//D/8f/y//P/9P/1//b/9//4//n/+v/7//z//f/+////",

  // Short error buzz
  error: "data:audio/wav;base64,UklGRjYBAABXQVZFZm10IBAAAAABAAEARKwAAIhYAQACABAAZGF0YRIBAACAf4B/gH+Af4B/gH+Af4B/gH+Af4B/gH+Af4B/gH+Af4B/f3+Af4B/gH+Af4B/gH+Af4B/gH+Af4B/gH9/f4B/gH+Af4B/gH+Af4B/gH+Af4B/gH+Af39/gH+Af4B/gH+Af4B/gH+Af4B/gH+Af4B/f3+Af4B/gH+Af4B/gH+Af4B/gH+Af4B/gH9/f4B/gH+Af4B/gH+Af4B/gH+Af4B/gH+Af39/gH+Af4B/gH+Af4B/gH+Af4B/gH+Af4B/f3+Af4B/gH+Af4B/gH+Af4B/gH+Af4B/gH9/f4B/gH+Af4B/gH+Af4B/gH+Af4B/gH+Af39/gH+Af4B/gH+Af4B/gH+Af4B/gH+Af4B/",

  // Soft notification ping
  notification: "data:audio/wav;base64,UklGRmoBAABXQVZFZm10IBAAAAABAAEARKwAAIhYAQACABAAZGF0YUYBAAB/f39/f39/f39/gICAgICBgYGCgoKDg4OEhISFhYWGhoaHh4eIiIiJiYmKioqLi4uMjIyNjY2Ojo6Pj4+QkJCRkZGSkpKTk5OUlJSVlZWWlpaXl5eYmJiZmZmampqbm5ucnJydnZ2enp6fn5+goKChoaGioqKjo6OkpKSlpaWmpqanp6eoqKipqamqqqqrq6usrKytra2urq6vr6+wsLCxsbGysrKzs7O0tLS1tbW2tra3t7e4uLi5ubm6urq7u7u8vLy9vb2+vr6/v7/AwMDBwcHCwsLDw8PExMTFxcXGxsbHx8fIyMjJycnKysrLy8vMzMzNzc3Ozs7Pz8/Q0NDR0dHS0tLT09PU1NTV1dXW1tbX19fY2NjZ2dna2trb29vc3Nzd3d3e3t7f39/g4ODh4eHi4uLj4+Pk5OTl5eXm5ubn5+fo6Ojp6enq6urr6+vs7Ozt7e3u7u7v7+/w8PDx8fHy8vLz8/P09PT19fX29vb39/f4+Pj5+fn6+vr7+/v8/Pz9/f3+/v7///8=",

  // Soft click
  click: "data:audio/wav;base64,UklGRiQAAABXQVZFZm10IBAAAAABAAEARKwAAIhYAQACABAAZGF0YQAAAAB/f39/",
};

export function useNotificationSound() {
  const audioContextRef = useRef<AudioContext | null>(null);
  const enabledRef = useRef(true);

  const getAudioContext = useCallback(() => {
    if (!audioContextRef.current) {
      audioContextRef.current = new (window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext)();
    }
    return audioContextRef.current;
  }, []);

  const playSound = useCallback(
    async (type: SoundType, volume = 0.3) => {
      if (!enabledRef.current) return;

      try {
        const audio = new Audio(SOUNDS[type]);
        audio.volume = Math.max(0, Math.min(1, volume));
        await audio.play();
      } catch (error) {
        // Fallback to Web Audio API beep
        try {
          const ctx = getAudioContext();
          const oscillator = ctx.createOscillator();
          const gainNode = ctx.createGain();

          oscillator.connect(gainNode);
          gainNode.connect(ctx.destination);

          const frequencies: Record<SoundType, number> = {
            success: 880,
            error: 220,
            notification: 660,
            click: 1000,
          };

          oscillator.frequency.value = frequencies[type];
          oscillator.type = type === "error" ? "square" : "sine";

          gainNode.gain.value = volume * 0.1;
          gainNode.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.15);

          oscillator.start();
          oscillator.stop(ctx.currentTime + 0.15);
        } catch {
          // Audio not available, silently fail
        }
      }
    },
    [getAudioContext]
  );

  const setEnabled = useCallback((enabled: boolean) => {
    enabledRef.current = enabled;
  }, []);

  return {
    playSound,
    playSuccess: () => playSound("success"),
    playError: () => playSound("error"),
    playNotification: () => playSound("notification"),
    playClick: () => playSound("click", 0.1),
    setEnabled,
  };
}
