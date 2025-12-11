"""
WebSocket Integration for Streamlit Dashboard

This module provides WebSocket functionality to receive real-time updates
from the backend without constant polling.
"""

import streamlit.components.v1 as components
import json

def create_websocket_component(user_pubkey, backend_url="ws://localhost:8080"):
    """
    Create a WebSocket component that connects to the backend
    and receives real-time updates.
    
    Args:
        user_pubkey: User's Solana wallet address
        backend_url: WebSocket URL (default: ws://localhost:8080)
    
    Returns:
        HTML component with WebSocket connection
    """
    
    ws_url = f"{backend_url}/ws/{user_pubkey}"
    
    # JavaScript code for WebSocket connection
    websocket_js = f"""
    <script>
        (function() {{
            // Prevent multiple WebSocket connections
            if (window.vaultWebSocket && window.vaultWebSocket.readyState === WebSocket.OPEN) {{
                console.log('⚠️ WebSocket already connected, reusing existing connection');
                return;
            }}
            
            // Wait for DOM to be ready
            let lastBalance = null;
            let notificationCount = 0;
            let reloadScheduled = false;
            
            // Connect to WebSocket
            const ws = new WebSocket('{ws_url}');
            window.vaultWebSocket = ws; // Store globally to prevent duplicates
            
            // Update status display - always find element dynamically
            function updateStatus(status, message) {{
                const statusText = document.getElementById('ws-status-text');
                const statusMessage = document.getElementById('ws-status-message');
                const container = document.getElementById('ws-status');
                
                if (statusText && statusMessage) {{
                    // Enhanced color scheme with high contrast and modern styling
                    let statusColor, bgGradient, borderColor, textColor, statusBg;
                    
                    if (status === 'connected') {{
                        statusColor = '#10b981'; // Bright emerald green
                        bgGradient = 'linear-gradient(135deg, rgba(16, 185, 129, 0.25) 0%, rgba(16, 185, 129, 0.15) 100%)';
                        borderColor = 'rgba(16, 185, 129, 0.6)';
                        textColor = '#ffffff'; // White text for maximum contrast
                        statusBg = 'rgba(16, 185, 129, 0.35)';
                    }} else if (status === 'error') {{
                        statusColor = '#ef4444'; // Bright red
                        bgGradient = 'linear-gradient(135deg, rgba(239, 68, 68, 0.25) 0%, rgba(239, 68, 68, 0.15) 100%)';
                        borderColor = 'rgba(239, 68, 68, 0.6)';
                        textColor = '#ffffff'; // White text
                        statusBg = 'rgba(239, 68, 68, 0.35)';
                    }} else if (status === 'disconnected') {{
                        statusColor = '#6b7280'; // Gray
                        bgGradient = 'linear-gradient(135deg, rgba(107, 114, 128, 0.25) 0%, rgba(107, 114, 128, 0.15) 100%)';
                        borderColor = 'rgba(107, 114, 128, 0.6)';
                        textColor = '#f3f4f6'; // Light gray text
                        statusBg = 'rgba(107, 114, 128, 0.35)';
                    }} else {{
                        statusColor = '#f59e0b'; // Amber/orange
                        bgGradient = 'linear-gradient(135deg, rgba(245, 158, 11, 0.25) 0%, rgba(245, 158, 11, 0.15) 100%)';
                        borderColor = 'rgba(245, 158, 11, 0.6)';
                        textColor = '#ffffff'; // White text
                        statusBg = 'rgba(245, 158, 11, 0.35)';
                    }}
                    
                    // Update status text badge
                    statusText.textContent = status.toUpperCase();
                    statusText.style.color = statusColor;
                    statusText.style.fontWeight = 'bold';
                    statusText.style.fontSize = '13px';
                    statusText.style.padding = '4px 12px';
                    statusText.style.backgroundColor = statusBg;
                    statusText.style.borderRadius = '6px';
                    statusText.style.border = `1px solid ${{borderColor}}`;
                    statusText.style.display = 'inline-block';
                    
                    // Update message text with high contrast white
                    statusMessage.textContent = message;
                    statusMessage.style.color = textColor;
                    statusMessage.style.fontSize = '14px';
                    statusMessage.style.lineHeight = '1.6';
                    statusMessage.style.fontWeight = '500';
                    statusMessage.style.textShadow = '0 1px 3px rgba(0,0,0,0.5)'; // Strong shadow for readability
                    statusMessage.style.marginTop = '4px';
                    
                    // Update container with modern gradient styling
                    if (container && container.firstElementChild) {{
                        const box = container.firstElementChild;
                        box.style.background = bgGradient;
                        box.style.border = `2px solid ${{borderColor}}`;
                        box.style.borderRadius = '10px';
                        box.style.padding = '14px 18px';
                        box.style.margin = '12px 0';
                        box.style.boxShadow = '0 4px 8px rgba(0,0,0,0.2)';
                        box.style.backdropFilter = 'blur(10px)';
                    }}
                    
                    console.log('✅ Status updated to:', status, '-', message);
                }} else {{
                    console.warn('Status elements not found, retrying in 100ms...');
                    setTimeout(() => updateStatus(status, message), 100);
                }}
            }}
            
            // Handle connection open
            ws.onopen = function() {{
                console.log('✅ WebSocket connection opened - readyState:', ws.readyState);
                // Update status immediately
                updateStatus('connected', 'Connected! Receiving real-time updates...');
            }};
            
            // Handle incoming messages
            ws.onmessage = function(event) {{
                try {{
                    const message = JSON.parse(event.data);
                    console.log('📨 WebSocket message received:', message);
                    console.log('📋 Event type:', message.event);
                    console.log('📊 Event data:', message.data);
                    
                    // Handle different event types
                    if (message.event === 'balance_update' || message.event === 'balanceUpdate') {{
                        const data = message.data;
                        const totalBalance = data.totalBalance || 0;
                        const availableBalance = data.availableBalance || 0;
                        const lockedBalance = data.lockedBalance || 0;
                        
                        const totalBalanceFormatted = (totalBalance / 1000000).toFixed(2);
                        const availableBalanceFormatted = (availableBalance / 1000000).toFixed(2);
                        const lockedBalanceFormatted = (lockedBalance / 1000000).toFixed(2);
                        
                        console.log('💰 Balance update received:', {{
                            total: totalBalanceFormatted,
                            available: availableBalanceFormatted,
                            locked: lockedBalanceFormatted,
                            rawData: data
                        }});
                        
                        // Store latest balance
                        lastBalance = {{
                            total: totalBalanceFormatted,
                            available: availableBalanceFormatted,
                            locked: lockedBalanceFormatted
                        }};
                        
                        // Show notification
                        notificationCount++;
                        updateStatus('connected', 
                            `💰 Balance updated! Total: ${{totalBalanceFormatted}} USDT, Available: ${{availableBalanceFormatted}} USDT, Locked: ${{lockedBalanceFormatted}} USDT`);
                        
                        // Dispatch custom event for Streamlit to handle
                        const balanceEvent = new CustomEvent('balanceUpdated', {{
                            detail: {{
                                totalBalance: totalBalance,
                                availableBalance: availableBalance,
                                lockedBalance: lockedBalance,
                                formattedTotal: totalBalanceFormatted,
                                formattedAvailable: availableBalanceFormatted,
                                formattedLocked: lockedBalanceFormatted
                            }}
                        }});
                        window.dispatchEvent(balanceEvent);
                        
                        // Schedule reload only once to avoid multiple reloads
                        if (!reloadScheduled) {{
                            reloadScheduled = true;
                            // Delay reload to ensure WebSocket stays open for other messages
                            // Use a longer delay (5 seconds) to avoid closing connection too quickly
                            setTimeout(() => {{
                                console.log('🔄 Triggering page reload to refresh balance display...');
                                reloadScheduled = false;
                                window.location.reload();
                            }}, 5000);
                        }} else {{
                            console.log('⏸️ Reload already scheduled, skipping...');
                        }}
                    }}
                    
                    if (message.event === 'transaction_confirmed' || message.event === 'transactionConfirmed') {{
                        const data = message.data;
                        const txType = data.transactionType.charAt(0).toUpperCase() + data.transactionType.slice(1);
                        const amount = (data.amount / 1000000).toFixed(2);
                        notificationCount++;
                        updateStatus('connected', 
                            `✅ Transaction confirmed: ${{txType}} of ${{amount}} USDT`);
                        
                        console.log('✅ Transaction confirmed:', txType, amount, 'USDT');
                        
                        // Dispatch custom event for Streamlit
                        const txEvent = new CustomEvent('transactionConfirmed', {{
                            detail: data
                        }});
                        window.dispatchEvent(txEvent);
                        
                        // Schedule reload only once
                        if (!reloadScheduled) {{
                            reloadScheduled = true;
                            setTimeout(() => {{
                                reloadScheduled = false;
                                if (window.parent && window.parent.postMessage) {{
                                    window.parent.postMessage({{
                                        type: 'streamlit:rerun'
                                    }}, '*');
                                }} else {{
                                    setTimeout(() => {{
                                        window.location.reload();
                                    }}, 2000);
                                }}
                            }}, 2000);
                        }}
                    }}
                    
                    // Handle collateral locked event
                    if (message.event === 'collateral_locked' || message.event === 'collateralLocked') {{
                        const data = message.data;
                        const amount = (data.amount / 1000000).toFixed(2);
                        const lockedBalance = (data.lockedBalance / 1000000).toFixed(2);
                        notificationCount++;
                        updateStatus('connected', 
                            `🔒 Collateral locked: ${{amount}} USDT (Total locked: ${{lockedBalance}} USDT)`);
                        
                        // Update balance
                        lastBalance = {{
                            total: (data.totalBalance / 1000000).toFixed(2),
                            available: (data.availableBalance / 1000000).toFixed(2),
                            locked: lockedBalance
                        }};
                        
                        // Trigger refresh
                        const customEvent = new CustomEvent('balanceUpdated', {{
                            detail: {{
                                totalBalance: data.totalBalance,
                                availableBalance: data.availableBalance,
                                lockedBalance: data.lockedBalance
                            }}
                        }});
                        window.dispatchEvent(customEvent);
                    }}
                    
                    // Handle collateral unlocked event
                    if (message.event === 'collateral_unlocked' || message.event === 'collateralUnlocked') {{
                        const data = message.data;
                        const amount = (data.amount / 1000000).toFixed(2);
                        const lockedBalance = (data.lockedBalance / 1000000).toFixed(2);
                        notificationCount++;
                        updateStatus('connected', 
                            `🔓 Collateral unlocked: ${{amount}} USDT (Total locked: ${{lockedBalance}} USDT)`);
                        
                        // Update balance
                        lastBalance = {{
                            total: (data.totalBalance / 1000000).toFixed(2),
                            available: (data.availableBalance / 1000000).toFixed(2),
                            locked: lockedBalance
                        }};
                        
                        // Trigger refresh - reload page to get latest balance
                        setTimeout(() => {{
                            window.location.reload();
                        }}, 500);
                    }}
                    
                    // Handle collateral unlocked event
                    if (message.event === 'collateral_unlocked' || message.event === 'collateralUnlocked') {{
                        const data = message.data;
                        const amount = (data.amount / 1000000).toFixed(2);
                        const lockedBalance = (data.lockedBalance / 1000000).toFixed(2);
                        notificationCount++;
                        updateStatus('connected', 
                            `🔓 Collateral unlocked: ${{amount}} USDT (Total locked: ${{lockedBalance}} USDT)`);
                        
                        // Update balance
                        lastBalance = {{
                            total: (data.totalBalance / 1000000).toFixed(2),
                            available: (data.availableBalance / 1000000).toFixed(2),
                            locked: lockedBalance
                        }};
                        
                        // Trigger refresh - reload page to get latest balance
                        setTimeout(() => {{
                            window.location.reload();
                        }}, 500);
                    }}
                    
                    // Handle health update (welcome message from backend)
                    if (message.event === 'health_update' || message.event === 'healthUpdate') {{
                        const statusMsg = message.data && message.data.message 
                            ? message.data.message 
                            : 'Connected and listening for updates';
                        updateStatus('connected', statusMsg);
                        console.log('✅ Health update received:', message.data);
                    }}
                    
                }} catch (e) {{
                    console.error('Error parsing WebSocket message:', e);
                    console.error('Raw message:', event.data);
                }}
            }};
            
            // Handle errors
            ws.onerror = function(error) {{
                updateStatus('error', 'Connection error. Check backend is running.');
                console.error('WebSocket error:', error);
                console.error('WebSocket state:', ws.readyState);
            }};
            
            // Handle disconnection
            ws.onclose = function(event) {{
                console.log('WebSocket disconnected. Code:', event.code, 'Reason:', event.reason);
                if (event.code === 1000) {{
                    updateStatus('disconnected', 'Connection closed normally');
                }} else {{
                    updateStatus('disconnected', 'Connection closed unexpectedly. Attempting to reconnect...');
                    // Attempt to reconnect after 3 seconds
                    setTimeout(function() {{
                        location.reload(); // Reload to reconnect
                    }}, 3000);
                }}
            }};
            
            // Log connection state changes
            console.log('WebSocket initial state:', ws.readyState);
            console.log('WebSocket URL:', '{ws_url}');
            
            // Expose balance getter for Streamlit
            window.getLastBalance = function() {{
                return lastBalance;
            }};
            
            // Cleanup on page unload
            window.addEventListener('beforeunload', function() {{
                ws.close();
            }});
        }})();
    </script>
    <div id="ws-status" style="min-height: 85px;">
        <div style="padding: 14px 18px; border-radius: 10px; margin: 12px 0; background: linear-gradient(135deg, rgba(245, 158, 11, 0.25) 0%, rgba(245, 158, 11, 0.15) 100%); border: 2px solid rgba(245, 158, 11, 0.6); box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2); backdrop-filter: blur(10px);">
            <div style="display: flex; align-items: center; gap: 10px; margin-bottom: 8px;">
                <strong style="color: #ffffff; font-size: 15px; font-weight: 600; text-shadow: 0 1px 2px rgba(0,0,0,0.3);">🔴 WebSocket Status:</strong> 
                <span style="color: #f59e0b; font-weight: bold; font-size: 13px; padding: 4px 12px; background-color: rgba(245, 158, 11, 0.35); border-radius: 6px; border: 1px solid rgba(245, 158, 11, 0.6); display: inline-block;" id="ws-status-text">CONNECTING...</span>
            </div>
            <div style="color: #ffffff; font-size: 14px; line-height: 1.6; font-weight: 500; text-shadow: 0 1px 3px rgba(0,0,0,0.5); margin-top: 4px;" id="ws-status-message">Establishing connection...</div>
        </div>
    </div>
    """
    
    return components.html(websocket_js, height=120)

