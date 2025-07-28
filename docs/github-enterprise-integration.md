# GitHub Enterprise Integration

This document describes how to configure and use GitHub Enterprise Server integration with Vibe Kanban.

## Overview

Vibe Kanban supports both GitHub.com and GitHub Enterprise Server for creating pull requests from task attempts. This allows teams using GitHub Enterprise Server to seamlessly integrate with their existing GitHub infrastructure.

## Configuration

### 1. Access Settings

Navigate to the Settings page in Vibe Kanban by clicking the gear icon in the sidebar.

### 2. Configure GitHub Enterprise

In the "GitHub Integration" section, you'll find the following fields:

#### GitHub Enterprise URL (Optional)
- **Purpose**: Specify the API base URL for your GitHub Enterprise Server
- **Format**: `https://github.company.com/api/v3`
- **Example**: `https://github.acme.com/api/v3`
- **Note**: Leave empty if you're using GitHub.com

#### Personal Access Token
- **Purpose**: Authentication token for GitHub API access
- **Permissions Required**: `repo` scope for creating pull requests
- **Format**: `ghp_xxxxxxxxxxxxxxxxxxxx`

#### Default PR Base Branch
- **Purpose**: Default target branch for pull requests
- **Default**: `main`
- **Example**: `develop`, `master`, `main`

### 3. Creating Personal Access Tokens

#### For GitHub Enterprise Server:
1. Navigate to your GitHub Enterprise instance
2. Go to Settings → Developer settings → Personal access tokens
3. Click "Generate new token"
4. Select the `repo` scope
5. Copy the generated token

#### For GitHub.com:
1. Go to https://github.com/settings/tokens
2. Click "Generate new token"
3. Select the `repo` scope
4. Copy the generated token

## Usage

### Creating Pull Requests

Once configured, you can create pull requests from task attempts:

1. Complete a task attempt
2. Click the "Create PR" button in the task attempt details
3. Fill in the PR title and description
4. Select the target branch (defaults to your configured base branch)
5. Click "Create Pull Request"

The system will:
- Push your branch to the configured GitHub instance
- Create a pull request using the GitHub API
- Update the task attempt with PR information

### Supported Features

- ✅ Pull request creation
- ✅ Custom base branches
- ✅ PR status monitoring
- ✅ GitHub Enterprise Server support
- ✅ GitHub.com support

### Repository Requirements

Your project repository must:
- Be hosted on GitHub.com or your GitHub Enterprise Server
- Have the `origin` remote configured
- Be accessible with your personal access token

## Troubleshooting

### Common Issues

#### "Not a GitHub repository" Error
- Ensure your repository's `origin` remote points to a GitHub URL
- Verify the URL format matches GitHub patterns

#### Authentication Errors
- Check that your personal access token is valid
- Ensure the token has `repo` scope permissions
- Verify the token works with your GitHub Enterprise instance

#### API URL Issues
- Ensure the Enterprise URL includes `/api/v3` suffix
- Verify the URL is accessible from your network
- Check that HTTPS is properly configured

### URL Format Examples

#### Correct GitHub Enterprise URLs:
- `https://github.company.com/api/v3`
- `https://git.enterprise.org/api/v3`

#### Incorrect URLs:
- `https://github.company.com` (missing `/api/v3`)
- `https://github.company.com/api/v3/` (trailing slash)
- `http://github.company.com/api/v3` (should use HTTPS)

## Security Considerations

- Store personal access tokens securely
- Use tokens with minimal required permissions
- Regularly rotate access tokens
- Consider using GitHub Apps for organization-wide deployments

## Support

If you encounter issues with GitHub Enterprise integration:

1. Check the application logs for detailed error messages
2. Verify your network connectivity to the GitHub Enterprise instance
3. Test your personal access token with the GitHub API directly
4. Ensure your repository URL format is supported
