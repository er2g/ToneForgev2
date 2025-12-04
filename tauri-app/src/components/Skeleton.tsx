import "./Skeleton.css";

interface SkeletonProps {
  width?: string;
  height?: string;
  variant?: "text" | "circular" | "rectangular";
  className?: string;
}

export function Skeleton({
  width = "100%",
  height = "1rem",
  variant = "text",
  className = "",
}: SkeletonProps) {
  return (
    <div
      className={`skeleton skeleton-${variant} ${className}`}
      style={{ width, height }}
    />
  );
}

export function SkeletonMessage() {
  return (
    <div className="skeleton-message">
      <Skeleton width="60%" height="1.2rem" />
      <Skeleton width="90%" height="1rem" />
      <Skeleton width="75%" height="1rem" />
    </div>
  );
}

export function SkeletonFxList() {
  return (
    <div className="skeleton-fx-list">
      {[1, 2, 3].map((i) => (
        <div key={i} className="skeleton-fx-item">
          <Skeleton width="70%" height="1rem" />
          <Skeleton width="4rem" height="1.5rem" variant="rectangular" />
        </div>
      ))}
    </div>
  );
}

export function SkeletonChannels() {
  return (
    <div className="skeleton-channels">
      {[1, 2].map((i) => (
        <div key={i} className="skeleton-channel">
          <Skeleton width="50%" height="0.9rem" />
          <Skeleton width="80%" height="1rem" />
          <Skeleton width="40%" height="0.8rem" />
        </div>
      ))}
    </div>
  );
}
