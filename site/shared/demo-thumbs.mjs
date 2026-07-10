// Demo-card thumbnail renderer — injects gradient SVG illustrations into
// [data-thumb] elements so the gallery looks alive without binary images.
// Call renderThumbs() after DOM ready.

const THUMBS = {
  pointcloud: (g) => `
    <svg viewBox="0 0 200 130" preserveAspectRatio="xMidYMid slice" style="width:100%;height:100%">
      <rect width="200" height="130" fill="#0d1117"/>
      ${Array.from({length: 90}, (_, i) => {
        const x = 10 + Math.random() * 180, y = 10 + Math.random() * 110;
        const t = Math.random();
        return `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="${(0.8 + Math.random() * 1.6).toFixed(1)}" fill="url(#${g})" opacity="${(0.3 + Math.random() * 0.6).toFixed(2)}"/>`;
      }).join('')}
    </svg>`,
  cesium: () => `
    <svg viewBox="0 0 200 130" preserveAspectRatio="xMidYMid slice" style="width:100%;height:100%">
      <defs><radialGradient id="globe-g" cx="0.35" cy="0.35"><stop offset="0%" stop-color="#1c2330"/><stop offset="100%" stop-color="#07090f"/></radialGradient></defs>
      <rect width="200" height="130" fill="#07090f"/>
      <circle cx="100" cy="65" r="48" fill="url(#globe-g)" stroke="#00d4ff" stroke-width="1" opacity="0.9"/>
      <ellipse cx="100" cy="65" rx="48" ry="14" fill="none" stroke="#a78bfa" stroke-width="0.7" opacity="0.5"/>
      <ellipse cx="100" cy="65" rx="20" ry="48" fill="none" stroke="#00d4ff" stroke-width="0.7" opacity="0.4"/>
      ${[[80,55],[110,60],[95,75],[120,50],[70,70]].map(([x,y])=>`<circle cx="${x}" cy="${y}" r="1.4" fill="#3fb950"/>`).join('')}
    </svg>`,
  terrain: () => `
    <svg viewBox="0 0 200 130" preserveAspectRatio="xMidYMid slice" style="width:100%;height:100%">
      <rect width="200" height="130" fill="#0d1117"/>
      <polygon points="0,130 0,90 40,70 80,85 120,55 160,75 200,60 200,130" fill="#1c2330"/>
      <polygon points="0,130 0,90 40,70 80,85 120,55 160,75 200,60 200,130" fill="none" stroke="#00d4ff" stroke-width="0.8" opacity="0.6"/>
      <polygon points="0,130 0,105 40,90 80,100 120,75 160,90 200,80 200,130" fill="#161b22"/>
      <polygon points="0,130 0,105 40,90 80,100 120,75 160,90 200,80 200,130" fill="none" stroke="#a78bfa" stroke-width="0.6" opacity="0.5"/>
      <line x1="120" y1="55" x2="120" y2="130" stroke="#a78bfa" stroke-width="0.5" stroke-dasharray="2 3" opacity="0.5"/>
    </svg>`,
  workflow: () => `
    <svg viewBox="0 0 200 130" preserveAspectRatio="xMidYMid slice" style="width:100%;height:100%">
      <rect width="200" height="130" fill="#0d1117"/>
      ${[20,60,100,140,180].map((x,i)=>`<rect x="${x-12}" y="55" width="24" height="24" rx="4" fill="#161b22" stroke="${['#3fb950','#00d4ff','#a78bfa','#00d4ff','#3fb950'][i]}" stroke-width="1"/><text x="${x}" y="71" font-family="monospace" font-size="7" fill="#8b949e" text-anchor="middle">${['LAS','OCT','TIL','CES','OK'][i]}</text>${i<4?`<path d="M${x+12} 67 L${x+28} 67" stroke="#3a4a5f" stroke-width="1" marker-end=""/ >`:''}`).join('')}
    </svg>`,
  webgl: () => `
    <svg viewBox="0 0 200 130" preserveAspectRatio="xMidYMid slice" style="width:100%;height:100%">
      <rect width="200" height="130" fill="#0d1117"/>
      ${Array.from({length:60},()=>{const x=Math.random()*200,y=Math.random()*130;return `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="${(0.6+Math.random()*1.4).toFixed(1)}" fill="#00d4ff" opacity="${(0.2+Math.random()*0.5).toFixed(2)}"/>`;}).join('')}
      <circle cx="100" cy="65" r="40" fill="none" stroke="#a78bfa" stroke-width="0.5" opacity="0.4"/>
    </svg>`,
  worker: () => `
    <svg viewBox="0 0 200 130" preserveAspectRatio="xMidYMid slice" style="width:100%;height:100%">
      <rect width="200" height="130" fill="#0d1117"/>
      ${[0,1,2,3].map(i=>`<rect x="${20+i*42}" y="30" width="34" height="20" rx="3" fill="#161b22" stroke="#00d4ff" stroke-width="0.8"/><rect x="${20+i*42}" y="80" width="34" height="20" rx="3" fill="#161b22" stroke="#a78bfa" stroke-width="0.8"/>`).join('')}
      <rect x="83" y="62" width="34" height="10" rx="2" fill="#1c2330" stroke="#3fb950" stroke-width="0.8"/>
      ${[0,1,2,3].map(i=>`<line x1="${37+i*42}" y1="50" x2="${100}" y2="62" stroke="#3a4a5f" stroke-width="0.6"/><line x1="${100}" y1="72" x2="${37+i*42}" y2="80" stroke="#3a4a5f" stroke-width="0.6"/>`).join('')}
    </svg>`,
  gis: () => `
    <svg viewBox="0 0 200 130" preserveAspectRatio="xMidYMid slice" style="width:100%;height:100%">
      <rect width="200" height="130" fill="#0d1117"/>
      ${Array.from({length:50},()=>{const x=Math.random()*200,y=Math.random()*130;return `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="1.1" fill="#00d4ff" opacity="0.4"/>`;}).join('')}
      <rect x="60" y="35" width="70" height="50" fill="none" stroke="#a78bfa" stroke-width="1" stroke-dasharray="3 2"/>
      ${[[80,50],[90,60],[100,55],[110,70]].map(([x,y])=>`<circle cx="${x}" cy="${y}" r="2.2" fill="#3fb950"/>`).join('')}
    </svg>`,
  bench: () => `
    <svg viewBox="0 0 200 130" preserveAspectRatio="xMidYMid slice" style="width:100%;height:100%">
      <rect width="200" height="130" fill="#0d1117"/>
      <rect x="20" y="85" width="14" height="30" fill="#3fb950"/>
      <rect x="44" y="55" width="14" height="60" fill="#3fb950" opacity="0.8"/>
      <rect x="68" y="40" width="14" height="75" fill="#22d3ee"/>
      <rect x="92" y="30" width="14" height="85" fill="#00d4ff"/>
      <rect x="116" y="25" width="14" height="90" fill="#a78bfa"/>
      <rect x="140" y="70" width="14" height="45" fill="#f85149" opacity="0.85"/>
      <rect x="164" y="60" width="14" height="55" fill="#f85149" opacity="0.7"/>
      <line x1="15" y1="115" x2="185" y2="115" stroke="#2a3441" stroke-width="0.8"/>
    </svg>`,
};

export function renderThumbs() {
  const grad = `tg-${Math.random().toString(36).slice(2, 7)}`;
  document.querySelectorAll('[data-thumb]').forEach(el => {
    const key = el.dataset.thumb;
    const fn = THUMBS[key];
    if (!fn) return;
    el.innerHTML = fn === THUMBS.pointcloud
      ? `<svg width="0" height="0"><defs><linearGradient id="${grad}" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="#00d4ff"/><stop offset="100%" stop-color="#a78bfa"/></linearGradient></defs></svg>${fn(grad)}`
      : fn();
  });
}
