// The one picture the developers page needs: what happens, and where. Speech
// goes in, a transcript and a filled form come out, and the box around all of
// it is the user's own device — nothing crosses its edge. Inline SVG, the
// site's tokens, no image download (skills/design.md).
export default function Pipeline({ className = '' }: { className?: string }) {
  const box = 'fill-white stroke-[#e2e7e3]';
  const label = 'fill-[#10212f] text-[13px] font-semibold';
  const sub = 'fill-[#66767f] text-[11px]';
  return (
    <svg
      viewBox="0 0 720 260"
      role="img"
      aria-label="Speech goes in on the device; a transcript and a filled form come out; nothing leaves the device"
      className={`w-full h-auto ${className}`}
    >
      {/* the device */}
      <rect x="14" y="14" width="692" height="232" rx="26" className="fill-[#f1f4f1] stroke-[#137a5b]" strokeWidth="1.5" strokeDasharray="6 6" />
      <text x="36" y="44" className="fill-[#137a5b] text-[12px] font-semibold tracking-[0.12em]">THE USER&apos;S DEVICE · PHONE OR LAPTOP · NO INTERNET NEEDED</text>

      {/* microphone */}
      <g transform="translate(40,80)">
        <rect width="150" height="110" rx="16" className={box} strokeWidth="1" />
        <circle cx="75" cy="40" r="16" className="fill-none stroke-[#137a5b]" strokeWidth="2.5" />
        <rect x="69" y="28" width="12" height="20" rx="6" className="fill-[#137a5b]" />
        <text x="75" y="82" textAnchor="middle" className={label}>Speech</text>
        <text x="75" y="98" textAnchor="middle" className={sub}>16 kHz audio, any language hint</text>
      </g>

      {/* arrow */}
      <path d="M196 135 H236" className="stroke-[#66767f]" strokeWidth="2" markerEnd="url(#arrow)" />

      {/* the runtime */}
      <g transform="translate(244,64)">
        <rect width="232" height="142" rx="18" className="fill-[#e4f2eb] stroke-[#137a5b]" strokeWidth="1.5" />
        <text x="116" y="34" textAnchor="middle" className="fill-[#10212f] text-[15px] font-bold">Verdure runtime</text>
        <text x="116" y="60" textAnchor="middle" className={sub}>picks the model this device can run</text>
        <text x="116" y="80" textAnchor="middle" className={sub}>whisper.cpp → transcript</text>
        <text x="116" y="100" textAnchor="middle" className={sub}>llama.cpp → JSON that matches your schema</text>
        <text x="116" y="126" textAnchor="middle" className="fill-[#137a5b] text-[11px] font-semibold">one library · one licence token · offline</text>
      </g>

      {/* arrows out */}
      <path d="M482 110 H522" className="stroke-[#66767f]" strokeWidth="2" markerEnd="url(#arrow)" />
      <path d="M482 160 H522" className="stroke-[#66767f]" strokeWidth="2" markerEnd="url(#arrow)" />

      {/* outputs */}
      <g transform="translate(530,78)">
        <rect width="150" height="54" rx="12" className={box} strokeWidth="1" />
        <text x="75" y="23" textAnchor="middle" className={label}>Transcript</text>
        <text x="75" y="41" textAnchor="middle" className={sub}>as the person speaks</text>
      </g>
      <g transform="translate(530,140)">
        <rect width="150" height="54" rx="12" className={box} strokeWidth="1" />
        <text x="75" y="23" textAnchor="middle" className={label}>{'{ "facts": [...] }'}</text>
        <text x="75" y="41" textAnchor="middle" className={sub}>valid against your schema</text>
      </g>

      <text x="360" y="232" textAnchor="middle" className="fill-[#66767f] text-[12px]">Nothing crosses the dotted line. Model files arrive once, from a server you choose.</text>

      <defs>
        <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
          <path d="M 0 0 L 10 5 L 0 10 z" className="fill-[#66767f]" />
        </marker>
      </defs>
    </svg>
  );
}
