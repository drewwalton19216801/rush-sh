// Theme Management
class ThemeManager {
  constructor() {
    this.theme = localStorage.getItem('theme') || 'dark';
    this.init();
  }

  init() {
    this.applyTheme();
    this.bindEvents();
  }

  applyTheme() {
    document.documentElement.setAttribute('data-theme', this.theme);
    const themeIcon = document.querySelector('.theme-icon');

    if (themeIcon) {
      themeIcon.textContent = this.theme === 'dark' ? '☀️' : '🌙';
    }
  }

  toggleTheme() {
    this.theme = this.theme === 'dark' ? 'light' : 'dark';
    localStorage.setItem('theme', this.theme);
    this.applyTheme();
  }

  bindEvents() {
    const themeToggle = document.getElementById('themeToggle');
    if (themeToggle) {
      themeToggle.addEventListener('click', () => this.toggleTheme());
    }
  }
}

// Mobile Menu Management
class MobileMenu {
  constructor() {
    this.sidebar = document.getElementById('sidebar');
    this.mobileMenuBtn = document.getElementById('mobileMenuBtn');
    this.isOpen = false;
    this.init();
  }

  init() {
    this.bindEvents();
  }

  toggle() {
    this.isOpen = !this.isOpen;
    this.sidebar.classList.toggle('open', this.isOpen);
    this.updateButton();
  }

  close() {
    this.isOpen = false;
    this.sidebar.classList.remove('open');
    this.updateButton();
  }

  updateButton() {
    const spans = this.mobileMenuBtn.querySelectorAll('span');

    if (this.isOpen) {
      spans[0].style.transform = 'rotate(45deg) translate(5px, 5px)';
      spans[1].style.opacity = '0';
      spans[2].style.transform = 'rotate(-45deg) translate(7px, -6px)';
    } else {
      spans[0].style.transform = 'none';
      spans[1].style.opacity = '1';
      spans[2].style.transform = 'none';
    }
  }

  bindEvents() {
    if (this.mobileMenuBtn) {
      this.mobileMenuBtn.addEventListener('click', () => this.toggle());
    }

    // Close menu when clicking outside
    document.addEventListener('click', (e) => {
      if (this.isOpen && !this.sidebar.contains(e.target) && !this.mobileMenuBtn.contains(e.target)) {
        this.close();
      }
    });

    // Close menu on window resize
    window.addEventListener('resize', () => {
      if (window.innerWidth > 768 && this.isOpen) {
        this.close();
      }
    });
  }
}

// Search Functionality
class SearchManager {
  constructor() {
    this.searchInput = document.getElementById('searchInput');
    this.init();
  }

  init() {
    this.bindEvents();
    this.buildSearchIndex();
  }

  bindEvents() {
    if (this.searchInput) {
      this.searchInput.addEventListener('input', (e) => this.handleSearch(e.target.value));
      this.searchInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          this.performSearch(e.target.value);
        }
      });
    }
  }

  buildSearchIndex() {
    // Simple search index - in a real implementation, this would be more sophisticated
    this.searchIndex = {
      'installation': ['installation', 'build', 'cargo', 'compile', 'setup'],
      'features': ['features', 'commands', 'builtins', 'expansion', 'variables'],
      'usage': ['usage', 'examples', 'commands', 'interactive', 'script'],
      'architecture': ['architecture', 'design', 'components', 'structure'],
      'compliance': ['posix', 'standards', 'compliance', 'specification']
    };
  }

  handleSearch(query) {
    if (query.length < 2) return;

    // Simple fuzzy search
    const results = this.performSearch(query);
    this.displaySearchSuggestions(results, query);
  }

  performSearch(query) {
    const results = [];

    Object.entries(this.searchIndex).forEach(([page, keywords]) => {
      const matches = keywords.filter(keyword =>
        keyword.toLowerCase().includes(query.toLowerCase())
      );

      if (matches.length > 0) {
        results.push({
          page: page,
          matches: matches,
          relevance: matches.length
        });
      }
    });

    return results.sort((a, b) => b.relevance - a.relevance);
  }

  displaySearchSuggestions(results, query) {
    // Remove existing suggestions
    const existingSuggestions = document.querySelector('.search-suggestions');
    if (existingSuggestions) {
      existingSuggestions.remove();
    }

    if (results.length === 0) return;

    const suggestions = document.createElement('div');
    suggestions.className = 'search-suggestions';
    suggestions.innerHTML = `
      <div class="search-suggestions-content">
        ${results.map(result => `
          <a href="${result.page}.html" class="search-suggestion">
            <span class="suggestion-page">${result.page}</span>
            <span class="suggestion-matches">${result.matches.join(', ')}</span>
          </a>
        `).join('')}
      </div>
    `;

    this.searchInput.parentElement.appendChild(suggestions);
  }
}

// Smooth Scrolling for Anchor Links
class SmoothScroll {
  constructor() {
    this.init();
  }

  init() {
    document.querySelectorAll('a[href^="#"]').forEach(anchor => {
      anchor.addEventListener('click', (e) => {
        e.preventDefault();
        const target = document.querySelector(anchor.getAttribute('href'));
        if (target) {
          target.scrollIntoView({
            behavior: 'smooth',
            block: 'start'
          });
        }
      });
    });
  }
}

// Copy Code Blocks
class CodeBlockManager {
  constructor() {
    this.init();
  }

  init() {
    document.querySelectorAll('.code-block').forEach(block => {
      this.addCopyButton(block);
    });
  }

  addCopyButton(block) {
    const copyButton = document.createElement('button');
    copyButton.className = 'copy-button';
    copyButton.innerHTML = '📋';
    copyButton.title = 'Copy code';

    copyButton.addEventListener('click', async () => {
      const code = block.querySelector('code');
      if (code) {
        try {
          await navigator.clipboard.writeText(code.textContent);
          copyButton.innerHTML = '✅';
          copyButton.title = 'Copied!';

          setTimeout(() => {
            copyButton.innerHTML = '📋';
            copyButton.title = 'Copy code';
          }, 2000);
        } catch (err) {
          // Fallback for older browsers
          const textArea = document.createElement('textarea');
          textArea.value = code.textContent;
          document.body.appendChild(textArea);
          textArea.select();

          try {
            document.execCommand('copy');
            copyButton.innerHTML = '✅';
            copyButton.title = 'Copied!';

            setTimeout(() => {
              copyButton.innerHTML = '📋';
              copyButton.title = 'Copy code';
            }, 2000);
          } catch (fallbackErr) {
            copyButton.innerHTML = '❌';
            copyButton.title = 'Copy failed';
          }

          document.body.removeChild(textArea);
        }
      }
    });

    block.style.position = 'relative';
    block.appendChild(copyButton);
  }
}

// Table of Contents Generation
class TableOfContents {
  constructor() {
    this.init();
  }

  init() {
    const sections = document.querySelectorAll('section[id]');
    if (sections.length === 0) return;

    this.createTOC(sections);
  }

  createTOC(sections) {
    const toc = document.createElement('div');
    toc.className = 'table-of-contents';
    toc.innerHTML = `
      <h3>📋 Table of Contents</h3>
      <ul class="toc-list">
        ${Array.from(sections).map(section => `
          <li class="toc-item">
            <a href="#${section.id}" class="toc-link">${section.querySelector('h2')?.textContent || section.id}</a>
          </li>
        `).join('')}
      </ul>
    `;

    // Insert TOC after the first section or at the top of main content
    const firstSection = document.querySelector('section');
    if (firstSection) {
      firstSection.insertBefore(toc, firstSection.firstChild);
    }
  }
}

// Performance Metrics Display
class PerformanceMetrics {
  constructor() {
    this.init();
  }

  init() {
    // Add loading states and performance metrics where appropriate
    this.addLoadTime();
  }

  addLoadTime() {
    window.addEventListener('load', () => {
      const loadTime = performance.now();
      if (loadTime < 100) {
        console.log(`🚀 Documentation loaded in ${Math.round(loadTime)}ms`);
      }
    });
  }
}

// Accessibility Enhancements
class AccessibilityManager {
  constructor() {
    this.init();
  }

  init() {
    this.addSkipLink();
    this.enhanceKeyboardNavigation();
  }

  addSkipLink() {
    const skipLink = document.createElement('a');
    skipLink.href = '#main-content';
    skipLink.textContent = 'Skip to main content';
    skipLink.className = 'skip-link';
    skipLink.style.cssText = `
      position: absolute;
      top: -40px;
      left: 6px;
      background: var(--accent-primary);
      color: white;
      padding: 8px;
      text-decoration: none;
      border-radius: 4px;
      z-index: 1000;
      transition: top 0.3s;
    `;

    skipLink.addEventListener('focus', () => {
      skipLink.style.top = '6px';
    });

    skipLink.addEventListener('blur', () => {
      skipLink.style.top = '-40px';
    });

    document.body.insertBefore(skipLink, document.body.firstChild);
  }

  enhanceKeyboardNavigation() {
    // Add proper focus indicators and keyboard navigation
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Tab') {
        document.body.classList.add('keyboard-navigation');
      }
    });

    document.addEventListener('mousedown', () => {
      document.body.classList.remove('keyboard-navigation');
    });
  }
}

// Initialize all managers when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
  new ThemeManager();
  new MobileMenu();
  new SearchManager();
  new SmoothScroll();
  new CodeBlockManager();
  new TableOfContents();
  new PerformanceMetrics();
  new AccessibilityManager();

  // Add some CSS for search suggestions
  const style = document.createElement('style');
  style.textContent = `
    .search-suggestions {
      position: absolute;
      top: 100%;
      left: 0;
      right: 0;
      background: var(--bg-primary);
      border: 1px solid var(--border-primary);
      border-radius: var(--radius-md);
      box-shadow: var(--shadow-lg);
      z-index: 1000;
      margin-top: 0.25rem;
    }

    .search-suggestions-content {
      max-height: 300px;
      overflow-y: auto;
    }

    .search-suggestion {
      display: block;
      padding: 0.75rem 1rem;
      color: var(--text-primary);
      text-decoration: none;
      border-bottom: 1px solid var(--border-primary);
      transition: background-color 0.2s ease;
    }

    .search-suggestion:last-child {
      border-bottom: none;
    }

    .search-suggestion:hover {
      background-color: var(--bg-secondary);
    }

    .suggestion-page {
      font-weight: 600;
      color: var(--accent-primary);
    }

    .suggestion-matches {
      font-size: 0.875rem;
      color: var(--text-muted);
    }

    .copy-button {
      position: absolute;
      top: 0.5rem;
      right: 0.5rem;
      background: var(--bg-tertiary);
      border: 1px solid var(--border-primary);
      border-radius: var(--radius-sm);
      padding: 0.25rem 0.5rem;
      font-size: 0.75rem;
      cursor: pointer;
      opacity: 0;
      transition: opacity 0.2s ease;
    }

    .code-block:hover .copy-button {
      opacity: 1;
    }

    .table-of-contents {
      background: var(--bg-secondary);
      border: 1px solid var(--border-primary);
      border-radius: var(--radius-lg);
      padding: 1.5rem;
      margin-bottom: 2rem;
    }

    .toc-list {
      list-style: none;
      padding-left: 0;
    }

    .toc-item {
      margin-bottom: 0.5rem;
    }

    .toc-link {
      color: var(--text-secondary);
      text-decoration: none;
      font-size: 0.875rem;
      display: block;
      padding: 0.25rem 0;
      transition: color 0.2s ease;
    }

    .toc-link:hover {
      color: var(--accent-primary);
    }

    .keyboard-navigation *:focus {
      outline: 2px solid var(--accent-primary);
      outline-offset: 2px;
    }
  `;
  document.head.appendChild(style);
});

// Export for potential external use
window.DocsUtils = {
  ThemeManager,
  MobileMenu,
  SearchManager,
  SmoothScroll,
  CodeBlockManager,
  TableOfContents,
  PerformanceMetrics,
  AccessibilityManager
};