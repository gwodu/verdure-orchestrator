'use client';

import { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';

export default function Home() {
  const [scrollY, setScrollY] = useState(0);
  const [animate, setAnimate] = useState(false);
  const router = useRouter();

  useEffect(() => {
    // Only run the scroll-driven reveal when the user is fine with motion.
    // Until this runs (SSR / pre-hydration / no-JS) and whenever reduced motion
    // is preferred, `animate` stays false and every section renders fully
    // visible and static — content is never hidden behind an animation.
    // See ../../skills/design.md, "The visual self-review loop".
    const motionOk = !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    setAnimate(motionOk);
    if (!motionOk) return;
    const handleScroll = () => setScrollY(window.scrollY);
    window.addEventListener('scroll', handleScroll, { passive: true });
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  const handleStartClick = () => {
    router.push('/start');
  };

  return (
    <div className="bg-slate-800 text-white min-h-screen">
      {/* Hero Section */}
      <section className="min-h-screen flex flex-col items-center justify-center px-6 sm:px-12">
        <div 
          className="text-center transition-all duration-700"
          style={animate ? {
            opacity: Math.max(1 - scrollY / 400, 0),
            transform: `translateY(${scrollY * 0.3}px)`
          } : undefined}
        >
          <h1 className="text-6xl sm:text-7xl md:text-8xl lg:text-9xl font-bold text-emerald-400 mb-6 tracking-tight">
            VERDURE
          </h1>
          <p className="text-xl sm:text-2xl md:text-3xl text-slate-200 mb-12 font-light">
            building your personal AI
          </p>
          <button
            onClick={handleStartClick}
            className="bg-emerald-400 text-slate-900 px-8 py-4 rounded-full text-lg font-semibold hover:bg-emerald-300 transition-all duration-300 hover:scale-105 shadow-lg hover:shadow-emerald-400/50"
          >
            Run your own local AI →
          </button>
          <p className="mt-4 text-sm text-slate-400">
            60 seconds. No signup until the end.
          </p>
        </div>
      </section>

      {/* About Section */}
      <section className="min-h-screen flex items-center justify-center px-6 sm:px-12 py-20">
        <div 
          className="max-w-4xl transition-all duration-700"
          style={animate ? {
            opacity: Math.min(Math.max((scrollY - 300) / 400, 0), 1),
            transform: `translateY(${Math.max(50 - (scrollY - 300) / 8, 0)}px)`
          } : undefined}
        >
          <h2 className="text-4xl sm:text-5xl md:text-6xl font-bold text-emerald-400 mb-8 text-center">
            About Verdure
          </h2>
          <div className="text-lg sm:text-xl md:text-2xl text-slate-200 leading-relaxed space-y-6">
            <p>
              AI is becoming part of everyday life — but at a cost. The things you type are sent off to servers you don&rsquo;t control, feeding models you can&rsquo;t see.
            </p>
            <p className="text-emerald-400 font-semibold">
              Verdure is personal AI that runs on your own device, not in the cloud — so your information never leaves your hands.
            </p>
            <p>
              We&rsquo;re building it to run entirely on your phone. Everything happens right there in your pocket: private by default, fast because nothing has to travel to a server and back, and far cheaper than anything that leans on the cloud.
            </p>
            <p>
              As you use it, Verdure learns what matters to you. Imagine waking up to a phone that has already sorted your email and messages down to the few things you actually need to see.
            </p>
            <p>
              And it grows with you. Over time your Verdure does more — booking the trips you&rsquo;d actually choose, or preparing your tax return — always right on your device, never on someone else&rsquo;s servers.
            </p>
            <p className="text-emerald-400 font-semibold">
              AI that works for you — and answers only to you.
            </p>
          </div>
        </div>
      </section>

      {/* Founder Section */}
      <section className="min-h-screen flex items-center justify-center px-6 sm:px-12 py-20">
        <div 
          className="max-w-3xl transition-all duration-700"
          style={animate ? {
            opacity: Math.min(Math.max((scrollY - 1000) / 400, 0), 1),
            transform: `translateY(${Math.max(50 - (scrollY - 1000) / 8, 0)}px)`
          } : undefined}
        >
          <h2 className="text-4xl sm:text-5xl md:text-6xl font-bold text-emerald-400 mb-12 text-center">
            Meet the Founder
          </h2>
          <div className="flex flex-col items-center text-center">
            <div className="w-64 sm:w-72 aspect-[3/4] rounded-2xl bg-slate-800 border-2 border-emerald-400/30 mb-8 flex items-center justify-center overflow-hidden shadow-[0_0_40px_-15px_rgba(52,211,153,0.3)]">
              <img 
                src="/Founders-Portrait.jpg" 
                alt="Gesi Wodu"
                className="w-full h-full object-cover object-top"
              />
            </div>
            <h3 className="text-3xl sm:text-4xl font-bold text-white mb-2">
              Gesi Wodu
            </h3>
            <a
              href="https://www.linkedin.com/in/gesikeme-wodu"
              target="_blank"
              rel="noopener noreferrer"
              className="text-emerald-400 hover:text-emerald-300 transition-colors mb-8 inline-flex items-center gap-2 text-lg"
            >
              <svg className="w-6 h-6" fill="currentColor" viewBox="0 0 24 24">
                <path d="M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433c-1.144 0-2.063-.926-2.063-2.065 0-1.138.92-2.063 2.063-2.063 1.14 0 2.064.925 2.064 2.063 0 1.139-.925 2.065-2.064 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.222 0h.003z"/>
              </svg>
              Connect on LinkedIn
            </a>
            <p className="text-lg sm:text-xl md:text-2xl text-slate-200 leading-relaxed">
              I studied computer science at the birthplace of AI, the University of Toronto, and I am ready to use that knowledge to bring affordable and power efficient AI direct to everyday people doing good work.
            </p>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="py-12 text-center text-slate-400 border-t border-slate-700">
        <p className="text-sm sm:text-base">
          © {new Date().getFullYear()} Verdure. All rights reserved.
        </p>
      </footer>
    </div>
  );
}