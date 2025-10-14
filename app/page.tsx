'use client';

import { useState, useEffect } from 'react';

export default function Home() {
  const [scrollY, setScrollY] = useState(0);

  useEffect(() => {
    const handleScroll = () => setScrollY(window.scrollY);
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  const handleWaitlistClick = () => {
    // Redirect to Google Forms waitlist
    window.open('https://docs.google.com/forms/d/e/1FAIpQLSeDmxvTw0-t7sV6rxOx0Px5LpYlHTzN9W_PqYyuzN96-PJ32w/viewform?usp=sharing&ouid=113612881470791656438', '_blank');
  };

  return (
    <div className="bg-slate-800 text-white min-h-screen">
      {/* Hero Section */}
      <section className="min-h-screen flex flex-col items-center justify-center px-6 sm:px-12">
        <div 
          className="text-center transition-all duration-700"
          style={{
            opacity: Math.max(1 - scrollY / 400, 0),
            transform: `translateY(${scrollY * 0.3}px)`
          }}
        >
          <h1 className="text-6xl sm:text-7xl md:text-8xl lg:text-9xl font-bold text-emerald-400 mb-6 tracking-tight">
            VERDURE
          </h1>
          <p className="text-xl sm:text-2xl md:text-3xl text-slate-200 mb-12 font-light">
            building your personal AI
          </p>
          <button
            onClick={handleWaitlistClick}
            className="bg-emerald-400 text-slate-900 px-8 py-4 rounded-full text-lg font-semibold hover:bg-emerald-300 transition-all duration-300 hover:scale-105 shadow-lg hover:shadow-emerald-400/50"
          >
            Join Our Waitlist
          </button>
        </div>
      </section>

      {/* About Section */}
      <section className="min-h-screen flex items-center justify-center px-6 sm:px-12 py-20">
        <div 
          className="max-w-4xl transition-all duration-700"
          style={{
            opacity: Math.min(Math.max((scrollY - 300) / 400, 0), 1),
            transform: `translateY(${Math.max(50 - (scrollY - 300) / 8, 0)}px)`
          }}
        >
          <h2 className="text-4xl sm:text-5xl md:text-6xl font-bold text-emerald-400 mb-8 text-center">
            About Verdure
          </h2>
          <div className="text-lg sm:text-xl md:text-2xl text-slate-200 leading-relaxed space-y-6">
            <p>
              AI is becoming essential to daily life—but at what cost? Your information gets scattered across the internet, absorbed into massive models you can't see or control.
            </p>
            <p className="text-emerald-400 font-semibold">
              Verdure is an elegant solution to AI safety and privacy concerns.
            </p>
            <p>
              We are building personal AI that runs entirely offline on your phone. 
              It protects your information by keeping it on your phone, while it uses that information to provide you personalised services, faster, and more affordable than anything relying on distant servers.
            </p>
            <p>
              Verdure learns your preferences as you use it, delivering tailored experiences.
              Imagine, waking up to perfectly prioritized notifications because Verdure sorts your emails and messages, showing only what matters.
            </p>
            <p>
              Verdure isn't just an assistant—it's a platform for next-gen AI apps. 
              Like an AI Travel Agent that books flights based on your learned preferences. 
              An AI Tax Agent that prepares returns while keeping financial data locked on your device.
            </p>
            <p className="text-emerald-400 font-semibold">
              Welcome to AI that works for you.
            </p>
          </div>
        </div>
      </section>

      {/* Founder Section */}
      <section className="min-h-screen flex items-center justify-center px-6 sm:px-12 py-20">
        <div 
          className="max-w-3xl transition-all duration-700"
          style={{
            opacity: Math.min(Math.max((scrollY - 1000) / 400, 0), 1),
            transform: `translateY(${Math.max(50 - (scrollY - 1000) / 8, 0)}px)`
          }}
        >
          <h2 className="text-4xl sm:text-5xl md:text-6xl font-bold text-emerald-400 mb-12 text-center">
            Meet the Founder
          </h2>
          <div className="flex flex-col items-center text-center">
            <div className="w-48 h-48 sm:w-56 sm:h-56 rounded-full bg-slate-700 border-4 border-emerald-400 mb-6 flex items-center justify-center overflow-hidden">
              {/* Replace the src below with your actual image path */}
              <img 
                src="/Founders-Portrait.jpg" 
                alt="Gesi Wodu"
                className="w-full h-full object-cover"
              />
              {/* Fallback if image not found - remove the div below once you add your image */}
              {/* <div className="text-slate-500 text-6xl">👤</div> */}
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
          © 2025 Verdure. All rights reserved.
        </p>
      </footer>
    </div>
  );
}