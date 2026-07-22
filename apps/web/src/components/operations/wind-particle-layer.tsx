"use client";

import { useEffect, useRef } from "react";
import type { Map as MapLibreMap } from "maplibre-gl";
import type { PublicWindField } from "@/lib/public-atmosphere";
import { nearestWindSample, windVector } from "./wind-particle-field";

type Props = {
  map: MapLibreMap | null;
  field: PublicWindField | null;
  visible: boolean;
};

type Particle = { longitude: number; latitude: number; age: number; maxAge: number };

export function WindParticleLayer({ map, field, visible }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !map || !field || !visible) return;
    const context = canvas.getContext("2d");
    if (!context) return;
    const activeField = field;
    const bounds = sampleBounds(activeField);
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    let animationFrame = 0;
    let disposed = false;
    const particles = Array.from({ length: reduced ? 0 : 72 }, () => spawnParticle(bounds));

    function resize() {
      if (!canvas || !map) return;
      const container = map.getContainer();
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.max(1, Math.round(container.clientWidth * ratio));
      canvas.height = Math.max(1, Math.round(container.clientHeight * ratio));
      canvas.style.width = `${container.clientWidth}px`;
      canvas.style.height = `${container.clientHeight}px`;
      context?.setTransform(ratio, 0, 0, ratio, 0, 0);
      if (reduced) drawStaticField(context!, map, activeField);
    }

    function draw() {
      if (disposed || !canvas || !context || !map) return;
      context.globalCompositeOperation = "destination-in";
      context.fillStyle = "rgba(0, 0, 0, 0.9)";
      context.fillRect(0, 0, canvas.clientWidth, canvas.clientHeight);
      context.globalCompositeOperation = "source-over";
      for (const particle of particles) {
        if (particle.age > particle.maxAge || outside(particle, bounds)) resetParticle(particle, bounds);
        const sample = nearestWindSample(activeField.samples, particle.longitude, particle.latitude);
        const vector = windVector(sample);
        const start = map.project([particle.longitude, particle.latitude]);
        const latitudeScale = Math.max(0.25, Math.abs(Math.cos(particle.latitude * Math.PI / 180)));
        particle.longitude += vector.east * 0.0000028 / latitudeScale;
        particle.latitude += vector.north * 0.0000028;
        particle.age += 1;
        const end = map.project([particle.longitude, particle.latitude]);
        context.beginPath();
        context.moveTo(start.x, start.y);
        context.lineTo(end.x, end.y);
        context.strokeStyle = windColor(sample.speed_knots, particle.age / particle.maxAge);
        context.lineWidth = sample.speed_knots >= 50 ? 1.7 : 1.2;
        context.stroke();
      }
      animationFrame = requestAnimationFrame(draw);
    }

    resize();
    map.on("resize", resize);
    map.on("move", resize);
    if (!reduced) animationFrame = requestAnimationFrame(draw);
    return () => {
      disposed = true;
      cancelAnimationFrame(animationFrame);
      map.off("resize", resize);
      map.off("move", resize);
      context.clearRect(0, 0, canvas.width, canvas.height);
    };
  }, [field, map, visible]);

  return <canvas ref={canvasRef} className="wind-particle-canvas" aria-hidden="true" />;
}

function sampleBounds(field: PublicWindField) {
  const longitudes = field.samples.map((sample) => sample.longitude_degrees);
  const latitudes = field.samples.map((sample) => sample.latitude_degrees);
  return {
    west: Math.min(...longitudes), east: Math.max(...longitudes),
    south: Math.min(...latitudes), north: Math.max(...latitudes),
  };
}

function spawnParticle(bounds: ReturnType<typeof sampleBounds>): Particle {
  return {
    longitude: bounds.west + Math.random() * (bounds.east - bounds.west),
    latitude: bounds.south + Math.random() * (bounds.north - bounds.south),
    age: Math.floor(Math.random() * 80),
    maxAge: 80 + Math.floor(Math.random() * 70),
  };
}

function resetParticle(particle: Particle, bounds: ReturnType<typeof sampleBounds>) {
  Object.assign(particle, spawnParticle(bounds), { age: 0 });
}

function outside(particle: Particle, bounds: ReturnType<typeof sampleBounds>) {
  return particle.longitude < bounds.west || particle.longitude > bounds.east ||
    particle.latitude < bounds.south || particle.latitude > bounds.north;
}

function windColor(speed: number, progress: number) {
  const alpha = Math.max(0.15, Math.sin(progress * Math.PI) * 0.88);
  return speed >= 60 ? `rgba(255, 166, 92, ${alpha})` : speed >= 30
    ? `rgba(107, 183, 255, ${alpha})` : `rgba(92, 225, 185, ${alpha})`;
}

function drawStaticField(context: CanvasRenderingContext2D, map: MapLibreMap, field: PublicWindField) {
  context.clearRect(0, 0, context.canvas.width, context.canvas.height);
  context.strokeStyle = "rgba(107, 183, 255, .72)";
  context.fillStyle = "rgba(107, 183, 255, .85)";
  context.lineWidth = 1.3;
  for (const sample of field.samples) {
    const vector = windVector(sample);
    const start = map.project([sample.longitude_degrees, sample.latitude_degrees]);
    const magnitude = Math.hypot(vector.east, vector.north) || 1;
    const dx = vector.east / magnitude * 13;
    const dy = -vector.north / magnitude * 13;
    context.beginPath();
    context.moveTo(start.x - dx, start.y - dy);
    context.lineTo(start.x + dx, start.y + dy);
    context.stroke();
    context.beginPath();
    context.arc(start.x + dx, start.y + dy, 2, 0, Math.PI * 2);
    context.fill();
  }
}
