import QRCode from 'qrcode';

// A QR code for a link, drawn at build time as inline SVG — no image file to
// keep in step with lib/links.ts, and nothing fetched. For the person reading
// the Try page on a laptop whose phone is the device the build is for.
export default async function QrCode({ url, label, size = 112 }: { url: string; label: string; size?: number }) {
  const svg = await QRCode.toString(url, { type: 'svg', margin: 1, color: { dark: '#10212f', light: '#ffffff' } });
  return (
    <div
      role="img"
      aria-label={label}
      className="rounded-lg overflow-hidden bg-white p-1 flex-none"
      style={{ width: size, height: size }}
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}
