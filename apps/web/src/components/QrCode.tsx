import QRCode from 'qrcode';
import type React from 'react';
import { useEffect, useState } from 'react';

interface QrCodeProps {
  /** The text/URL to encode. */
  value: string;
  /** Rendered size in CSS pixels (square). Defaults to 160. */
  size?: number;
  className?: string;
}

/**
 * Renders `value` as a scannable QR code (PNG data URI, generated client-side —
 * nothing is sent anywhere to produce it). Used to pair a phone with the LAN
 * web-mode link without typing it in by hand.
 */
export function QrCode({ value, size = 160, className }: QrCodeProps): React.JSX.Element | null {
  const [dataUrl, setDataUrl] = useState<string | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setError(false);
    QRCode.toDataURL(value, { width: size, margin: 1 })
      .then((url) => {
        if (!cancelled) setDataUrl(url);
      })
      .catch(() => {
        if (!cancelled) setError(true);
      });
    return () => {
      cancelled = true;
    };
  }, [value, size]);

  if (error) return null;
  if (!dataUrl) {
    return (
      <div
        className={className}
        style={{ width: size, height: size }}
        aria-hidden="true"
      />
    );
  }

  return (
    <img
      src={dataUrl}
      alt={`QR code for ${value}`}
      width={size}
      height={size}
      className={className}
    />
  );
}
