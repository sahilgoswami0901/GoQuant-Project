# Quick Start: Deploy Documentation

## 🚀 Fastest Way: GitHub Pages (Free)

### Step 1: Build Documentation

```bash
cd backend
./scripts/build-docs.sh
```

### Step 2: Enable GitHub Pages

1. Push the `.github/workflows/docs.yml` file to your repository
2. Go to your GitHub repo → **Settings** → **Pages**
3. Under **Source**, select **GitHub Actions**
4. Save

### Step 3: Push Changes

```bash
git add .
git commit -m "Add documentation deployment"
git push
```

The workflow will automatically:
- Build documentation on every push
- Deploy to GitHub Pages
- Make it available at: `https://YOUR_USERNAME.github.io/YOUR_REPO/`

---

## 🌐 Alternative: Netlify (Also Free)

### Step 1: Build Documentation

```bash
cd backend
./scripts/build-docs.sh
```

### Step 2: Deploy

1. Go to [netlify.com](https://netlify.com) and sign up
2. Click **Add new site** → **Import an existing project**
3. Connect your GitHub repository
4. Netlify will automatically detect `netlify.toml`
5. Click **Deploy site**

Your docs will be live at: `https://YOUR_SITE.netlify.app`

---

## 📦 Manual Deployment

### Step 1: Build

```bash
cd backend
./scripts/build-docs.sh
```

### Step 2: Package

```bash
cd target/doc
tar -czf ../../docs.tar.gz .
```

### Step 3: Upload

Upload `docs.tar.gz` to any static hosting service:
- AWS S3 + CloudFront
- Google Cloud Storage
- Azure Static Web Apps
- Any web server

---

## 🧪 Test Locally

Before deploying, test locally:

```bash
cd backend/target/doc
python3 -m http.server 8000
```

Then visit: `http://localhost:8000`

---

## 📝 Notes

- Documentation is generated in `backend/target/doc/`
- The main entry point is usually `vault_backend/index.html`
- All assets (CSS, JS, images) are included automatically
- Documentation updates automatically on each push (with GitHub Actions)
