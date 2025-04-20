# Tau's Game Night

## Web Push

### Example Payload

```json
{
  "web_push": 8030,
  "notification": {
   	"title": "Tau's Game Night is happening on November 25 🥳",
  	"body": "You're warmly invited to the next Game Night on November 25, be sure to save the date :)",
  	"icon": "https://tau.garden/favicon.svg",
  	"navigation": "/play",
  	"requireInteraction": true,
  	"actions": [
  		{
  		  "action": "save",
  		  "title": "Save to Calendar",
  			"navigation": "/play/event.ics"
  		}
  	]
  },
}
```
